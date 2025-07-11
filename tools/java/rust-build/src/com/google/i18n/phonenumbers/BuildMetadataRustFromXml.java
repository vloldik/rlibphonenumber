/*
 *  Copyright (C) 2011 The Libphonenumber Authors
 *  Copyright (C) 2025 The Kashin Vladislav (modified)
 * 
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *  http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

package com.google.i18n.phonenumbers;

import com.google.i18n.phonenumbers.RustMetadataGenerator.Type;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileNotFoundException;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.io.OutputStreamWriter;
import java.nio.charset.Charset;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * This class generates the Rust code representation of the provided XML metadata file. It lets us
 * embed metadata directly in a native binary. We link the object resulting from the compilation of
 * the code emitted by this class with the Rust rlibphonenumber library.
 *
 * @author Philippe Liard
 * @author David Beaumont
 * 
 * @author Kashin Vladislav (modified for Rust code generation)
 */
public class BuildMetadataRustFromXml extends Command {

  /** An enum encapsulating the variations of metadata that we can produce. */
  public enum Variant {
    /** The default 'full' variant which contains all the metadata. */
    FULL("%s"),
    /** The test variant which contains fake data for tests. */
    TEST("test_%s"),
    /**
     * The lite variant contains the same metadata as the full version but excludes any example
     * data. This is typically used for clients with space restrictions.
     */
    LITE("lite_%s");

    private final String template;

    private Variant(String template) {
      this.template = template;
    }

    /**
     * Returns the basename of the type by adding the name of the current variant. The basename of
     * a Type is used to determine the name of the source file in which the metadata is defined.
     *
     * <p>Note that when the variant is {@link Variant#FULL} this method just returns the type name.
     */
    public String getBasename(Type type) {
      return String.format(template, type);
    }

    /**
     * Parses metadata variant name. By default (for a name of {@code ""} or {@code null}) we return
     * {@link Variant#FULL}, otherwise we match against the variant name (either "test" or "lite").
     */
    public static Variant parse(String variantName) {
      if ("test".equalsIgnoreCase(variantName)) {
        return Variant.TEST;
      } else if ("lite".equalsIgnoreCase(variantName)) {
        return Variant.LITE;
      } else if (variantName == null || variantName.length() == 0) {
        return Variant.FULL;
      } else {
        return null;
      }
    }
  }

  /**
   * An immutable options class for parsing and representing the command line options for this
   * command.
   */
  // @VisibleForTesting
  static final class Options {
    private static final Pattern BASENAME_PATTERN =
        Pattern.compile("(?:(test|lite)_)?([a-z_]+)");
    private static final Pattern CONSTANT_NAME_PATTERN = 
        Pattern.compile("--const-name[ =]([a-zA-Z_]+)");
    private static final String DEFAULT_METADATA_CONSTANT_NAME = "METADATA";
    public static Options parse(String commandName, String[] argsArray) {
      ArrayList args = new ArrayList(Arrays.asList(argsArray));
      String constantName = DEFAULT_METADATA_CONSTANT_NAME;
      if (args.size() == 5) {
        for (int i = 0; i < args.size(); i++) {
          String arg = args.get(i).toString();
          Matcher matcher = CONSTANT_NAME_PATTERN.matcher(arg.toString());
          if (matcher.matches()) {
            constantName = matcher.group(1);
            args.remove(arg);
            break;
          }
        }
      }
      if (args.size() == 4) {
        String inputXmlFilePath = args.get(1).toString();
        String outputDirPath = args.get(2).toString();
        Matcher basenameMatcher = BASENAME_PATTERN.matcher(args.get(3).toString());
        if (basenameMatcher.matches()) {
          Variant variant = Variant.parse(basenameMatcher.group(1));
          Type type = Type.parse(basenameMatcher.group(2));
          if (type != null && variant != null) {
            return new Options(inputXmlFilePath, outputDirPath, type, variant, constantName);
          }
        }
      }
      throw new IllegalArgumentException(String.format(
          "Usage: %s <inputXmlFile> <outputDir> <output ( <type> | test_<type> | lite_<type> ) " +
          "[--const-name <nameOfMetadataConstant>] \n" +
          "       where <type> is one of: %s",
          commandName, Arrays.asList(Type.values())));
    }

    // File path where the XML input can be found.
    private final String inputXmlFilePath;
    // Output directory where the generated files will be saved.
    private final String outputDirPath;
    private final Type type;
    private final Variant variant;
    private final String constantName;

    private Options(String inputXmlFilePath, String outputDirPath, Type type, Variant variant, String constantName) {
      this.inputXmlFilePath = inputXmlFilePath;
      this.outputDirPath = outputDirPath;
      this.type = type;
      this.variant = variant;
      this.constantName = constantName;
    }

    public String getInputFilePath() {
      return inputXmlFilePath;
    }

    public String getOutputDir() {
      return outputDirPath;
    }

    public Type getType() {
      return type;
    }

    public Variant getVariant() {
      return variant;
    }

    public String getConstantName() {
      return constantName;
    }
  }

  @Override
  public String getCommandName() {
    return "BuildMetadataRustFromXml";
  }

  /**
   * Generates Rust source file to represent the metadata specified by this command's
   * arguments. The metadata XML file is read and converted to a byte array before being written
   * into a Rust source file as a static data array.
   *
   * @return  true if the generation succeeded.
   */
  @Override
  public boolean start() {
    try {
      Options opt = Options.parse(getCommandName(), getArgs());
      byte[] data = loadMetadataBytes(opt.getInputFilePath(), opt.getVariant() == Variant.LITE);
      RustMetadataGenerator metadata = RustMetadataGenerator.create(opt.getType(), data, opt.constantName);

      // TODO: Consider adding checking for correctness of file paths and access.
      OutputStream headerStream = null;
      OutputStream sourceStream = null;
      try {
        File dir = new File(opt.getOutputDir());
        sourceStream = openSourceStream(dir);
        metadata.outputSourceFile(new OutputStreamWriter(sourceStream, UTF_8));
      } finally {
        FileUtils.closeFiles(headerStream, sourceStream);
      }
      return true;
    } catch (IOException e) {
      System.err.println(e.getMessage());
    } catch (RuntimeException e) {
      System.err.println(e.getMessage());
    }
    return false;
  }

  /** Loads the metadata XML file and converts its contents to a byte array. */
  private byte[] loadMetadataBytes(String inputFilePath, boolean liteMetadata) {
    ByteArrayOutputStream out = new ByteArrayOutputStream();
    try {
      writePhoneMetadataCollection(inputFilePath, liteMetadata, out);
    } catch (Exception e) {
      // We cannot recover from any exceptions thrown here, so promote them to runtime exceptions.
      throw new RuntimeException(e);
    } finally {
      FileUtils.closeFiles(out);
    }
    return out.toByteArray();
  }

  // @VisibleForTesting
  void writePhoneMetadataCollection(
      String inputFilePath, boolean liteMetadata, OutputStream out) throws IOException, Exception {
    BuildMetadataFromXml.buildPhoneMetadataCollection(inputFilePath, liteMetadata, false)
        .writeTo(out);
  }

  // @VisibleForTesting
  OutputStream openSourceStream(File file) throws FileNotFoundException {
    return new FileOutputStream(file);
  }

  /** The charset in which our source and header files will be written. */
  private static final Charset UTF_8 = Charset.forName("UTF-8");
}

/*
 *  Copyright (C) 2012 The Libphonenumber Authors
 *  Copyright (C) 2025 Kashin Vladislav (modified)
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

import java.io.IOException;
import java.io.PrintWriter;
import java.io.Writer;
import java.util.Locale;

/**
 * Encapsulation of binary metadata created from XML to be included as static data in C++ source
 * files.
 *
 * @author David Beaumont
 * @author Philippe Liard
 * 
 * @author Kashin Vladislav (modified for Rust code generation)
 */
public final class RustMetadataGenerator {

  /**
   * The metadata type represents the known types of metadata and includes additional information
   * such as the copyright year. It is expected that the generated files will be named after the
   * {@link #toString} of their type.
   */
  public enum Type {
    /** The basic phone number metadata (expected to be written to metadata.[h/cc]). */
    METADATA(2011, 2025),
    /** The alternate format metadata (expected to be written to alternate_format.[h/cc]). */
    ALTERNATE_FORMAT(2012, 2025),
    /** Metadata for short numbers (expected to be written to short_metadata.[h/cc]). */
    SHORT_NUMBERS(2013, 2025);

    private final int copyrightYear;
    private final int copyrightSecondYear;

    private Type(int copyrightYear, int CopyrightSecondYear) {
      this.copyrightYear = copyrightYear;
      this.copyrightSecondYear = CopyrightSecondYear;
    }

    /** Returns the year in which this metadata type was first introduced. */
    public int getCopyrightYear() {
      return copyrightYear;
    }

     /** Returns the year in which this metadata type was modified for RUST. */
    public int getCopyrightSecondYear() {
      return copyrightSecondYear;
    }

    /**
     * Parses the type from a string case-insensitively.
     *
     * @return the matching Type instance or null if not matched.
     */
    public static Type parse(String typeName) {
      if (Type.METADATA.toString().equalsIgnoreCase(typeName)) {
        return Type.METADATA;
      } else if (Type.ALTERNATE_FORMAT.toString().equalsIgnoreCase(typeName)) {
        return Type.ALTERNATE_FORMAT;
      } else if (Type.SHORT_NUMBERS.toString().equalsIgnoreCase(typeName)) {
        return Type.SHORT_NUMBERS;
      } else {
        return null;
      }
    }
  }

  /**
   * Creates a metadata instance that can write C++ source and header files to represent this given
   * byte array as a static unsigned char array. Note that a direct reference to the byte[] is
   * retained by the newly created CppXmlMetadata instance, so the caller should treat the array as
   * immutable after making this call.
   */
  public static RustMetadataGenerator create(Type type, byte[] data, String constantName) {
    return new RustMetadataGenerator(type, data, constantName);
  }

  private final Type type;
  private final byte[] data;
  private final String constantName;

  private RustMetadataGenerator(Type type, byte[] data, String variableName) {
    this.type = type;
    this.data = data;
    this.constantName = variableName;
  }

  /**
   * Writes the source file for the Rust representation of the metadata - a static array
   * containing the data itself, to the given writer. Note that this method does not close the given
   * writer.
   */
  public void outputSourceFile(Writer out) throws IOException {
    // TODO: Consider outputting a load method to return the parsed proto directly.
    String dataLength = String.valueOf(data.length);


    PrintWriter pw = new PrintWriter(out);
    CopyrightNotice.writeTo(pw, type.getCopyrightYear(), type.getCopyrightSecondYear());
    pw.println("pub const "+constantName+": [u8; "+dataLength+"] = [");
    emitStaticArrayData(pw, data);
    pw.println("];");
    pw.flush();
  }

  /** Emits the Rust code corresponding to the binary metadata as a static byte array. */
  // @VisibleForTesting
  static void emitStaticArrayData(PrintWriter pw, byte[] data) {
    String separator = "  ";
    for (int i = 0; i < data.length; i++) {
      pw.print(separator);
      emitHexByte(pw, data[i]);
      separator = ((i + 1) % 13 == 0) ? ",\n  " : ", ";
    }
    pw.println();
  }

  /** Emits a single byte in the form 0xHH, where H is an upper case hex digit in [0-9A-F]. */
  private static void emitHexByte(PrintWriter pw, byte v) {
    pw.print("0x");
    pw.print(UPPER_HEX[(v & 0xF0) >>> 4]);
    pw.print(UPPER_HEX[v & 0xF]);
  }

  private static final char[] UPPER_HEX = "0123456789ABCDEF".toCharArray();
}

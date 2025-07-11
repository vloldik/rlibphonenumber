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

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import com.google.i18n.phonenumbers.BuildMetadataRustFromXml.Options;
import com.google.i18n.phonenumbers.BuildMetadataRustFromXml.Variant;
import com.google.i18n.phonenumbers.RustMetadataGenerator.Type;

import org.junit.Test;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.OutputStream;
import java.nio.charset.Charset;

/**
 * Tests the BuildMetadataCppFromXml implementation to make sure it parses command line options and
 * generates code correctly.
 */
public class BuildMetadataRustFromXmlTest {

  // Various repeated test strings and data.
  private static final String IGNORED = "IGNORED";
  private static final String OUTPUT_DIR = "output/dir";
  private static final String INPUT_PATH_XML = "input/path.xml";
  private static final byte[] TEST_DATA =
      new byte[] { (byte) 0xCA, (byte) 0xFE, (byte) 0xBA, (byte) 0xBE };
  private static final int TEST_DATA_LEN = TEST_DATA.length;
  private static final String TEST_CONSTANT_NAME = "METADATA";
  private static final String OUTPUT_DATA = "0xCA, 0xFE, 0xBA, 0xBE";

  @Test
  public void parseVariant() {
    assertNull(Variant.parse("xxx"));
    assertEquals(Variant.FULL, Variant.parse(null));
    assertEquals(Variant.FULL, Variant.parse(""));
    assertEquals(Variant.LITE, Variant.parse("lite"));
    assertEquals(Variant.TEST, Variant.parse("test"));
    assertEquals(Variant.LITE, Variant.parse("LITE"));
    assertEquals(Variant.TEST, Variant.parse("Test"));
  }

  @Test
  public void parseBadOptions() {
    try {
      BuildMetadataRustFromXml.Options.parse("MyCommand", new String[] { IGNORED });
      fail("Expected exception not thrown");
    } catch (IllegalArgumentException e) {
      assertTrue(e.getMessage().contains("MyCommand"));
    }
  }

  @Test
  public void parseGoodOptions() {
    Options opt = BuildMetadataRustFromXml.Options.parse("MyCommand",
        new String[] { IGNORED, INPUT_PATH_XML, OUTPUT_DIR, "test_alternate_format", "--const-name=" + TEST_CONSTANT_NAME });
    assertEquals(Type.ALTERNATE_FORMAT, opt.getType());
    assertEquals(Variant.TEST, opt.getVariant());
    assertEquals(INPUT_PATH_XML, opt.getInputFilePath());
    assertEquals(OUTPUT_DIR, opt.getOutputDir());
    assertEquals(TEST_CONSTANT_NAME, opt.getConstantName());
  }

  @Test
  public void generateMetadata() {
    String[] args = new String[] {
        IGNORED, INPUT_PATH_XML, OUTPUT_DIR, "metadata", "--const-name " + TEST_CONSTANT_NAME };
    // Most of the useful asserts are done in the mock class.
    MockedCommand command = new MockedCommand(
        INPUT_PATH_XML, false, OUTPUT_DIR, Type.METADATA, Variant.FULL, TEST_CONSTANT_NAME
    );
    command.setArgs(args);
    command.start();
    // Sanity check the captured data (asserting implicitly that the mocked methods were called).
    String sourceString = command.capturedSourceFile();
    assertTrue(sourceString.contains("pub const "+TEST_CONSTANT_NAME+": [u8; " + TEST_DATA_LEN + "] ="));
    assertTrue(sourceString.contains(OUTPUT_DATA));
    assertTrue(sourceString.contains("];"));
  }

  // no need test for metadata with other names since it's set with parameter

  /**
   * Manually mocked subclass of BuildMetadataCppFromXml which overrides all file related behavior
   * while asserting the validity of any parameters passed to the mocked methods. After starting
   * this command, the captured header and source file contents can be retrieved for testing.
   */
  static class MockedCommand extends BuildMetadataRustFromXml {
    private static final Charset UTF_8 = Charset.forName("UTF-8");
    private final String expectedInputFilePath;
    private final boolean expectedLiteMetadata;
    private final String expectedOutputDirPath;
    private final Type expectedType;
    private final Variant expectedVariant;
    private final String expectedConstantName;
    private final ByteArrayOutputStream sourceOut = new ByteArrayOutputStream();

    public MockedCommand(String expectedInputFilePath, boolean expectedLiteMetadata,
        String expectedOutputDirPath, Type expectedType, Variant expectedVariant,
        String expectedConstantName) {

      this.expectedInputFilePath = expectedInputFilePath;
      this.expectedLiteMetadata = expectedLiteMetadata;
      this.expectedOutputDirPath = expectedOutputDirPath;
      this.expectedType = expectedType;
      this.expectedConstantName = expectedConstantName;
      this.expectedVariant = expectedVariant;
    }
    @Override void writePhoneMetadataCollection(
        String inputFilePath, boolean liteMetadata, OutputStream out) throws Exception {
      assertEquals(expectedInputFilePath, inputFilePath);
      assertEquals(expectedLiteMetadata, liteMetadata);
      out.write(TEST_DATA, 0, TEST_DATA.length);
    }
    @Override OutputStream openSourceStream(File dir) {
      assertEquals(expectedOutputDirPath, dir.getPath());
      return sourceOut;
    }
    
    String capturedSourceFile() {
      return new String(sourceOut.toByteArray(), UTF_8);
    }
  }
}

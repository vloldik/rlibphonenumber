/*
 *  Copyright (C) 2012 The Libphonenumber Authors
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
import static org.junit.Assert.assertTrue;

import com.google.i18n.phonenumbers.RustMetadataGenerator.Type;

import org.junit.Test;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.PrintWriter;
import java.io.StringReader;
import java.io.StringWriter;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;

/**
 * Tests that the CppXmlMetadata class emits the expected source and header files for metadata.
 */
public class RustMetadataGeneratorTest {

  // 13 bytes per line, so have 16 bytes to test > 1 line (general case).
  // Use all hex digits in both nibbles to test hex formatting.
  private static final byte[] TEST_DATA = new byte[] {
      (byte) 0xF0, (byte) 0xE1, (byte) 0xD2, (byte) 0xC3,
      (byte) 0xB4, (byte) 0xA5, (byte) 0x96, (byte) 0x87,
      (byte) 0x78, (byte) 0x69, (byte) 0x5A, (byte) 0x4B,
      (byte) 0x3C, (byte) 0x2D, (byte) 0x1E, (byte) 0x0F,
  };
  private static final int TEST_DATA_LEN = TEST_DATA.length;
  private static final String TEST_CONSTANT_NAME = "METADATA";

  @Test
  public void emitStaticArrayData() {

    byte[] data = TEST_DATA;

    StringWriter writer = new StringWriter();
    RustMetadataGenerator.emitStaticArrayData(new PrintWriter(writer), data);

  }

  @Test
  public void outputSourceFile() throws IOException {
    byte[] data = new byte[] { (byte) 0xCA, (byte) 0xFE, (byte) 0xBA, (byte) 0xBE };
    String testDataLen = String.valueOf(data.length);
    RustMetadataGenerator metadata = RustMetadataGenerator.create(Type.ALTERNATE_FORMAT, data, TEST_CONSTANT_NAME);

    StringWriter writer = new StringWriter();
    metadata.outputSourceFile(writer);
    Iterator<String> lines = toLines(writer.toString()).iterator();
    // Sanity check that at least some of the expected lines are present.
    assertTrue(consumeUntil("pub const "+TEST_CONSTANT_NAME+": [u8; "+testDataLen+"] = [", lines));
    assertTrue(consumeUntil("  0xCA, 0xFE, 0xBA, 0xBE", lines));
    assertTrue(consumeUntil("];", lines));
  }

  /** Converts a string containing newlines into a list of lines. */
  private static List<String> toLines(String s) throws IOException {
    BufferedReader reader = new BufferedReader(new StringReader(s));
    List<String> lines = new ArrayList<String>();
    for (String line = reader.readLine(); line != null; line = reader.readLine()) {
      lines.add(line);
    }
    return lines;
  }

  /**
   * Consumes strings from the given iterator until the expected string is reached (it is also
   * consumed). If the expected string is not found, the iterator is exhausted and {@code false} is
   * returned.
   *
   * @return true if the expected string was found while consuming the iterator.
   */
  private static boolean consumeUntil(String expected, Iterator<String> it) {
    while (it.hasNext()) {
      if (it.next().equals(expected)) {
        return true;
      }
    }
    return false;
  }
}

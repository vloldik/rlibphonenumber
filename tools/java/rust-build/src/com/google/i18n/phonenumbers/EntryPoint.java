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

/**
 * Entry point class for C++ build tools.
 *
 * @author Philippe Liard
 * 
 * @author Kashin Vladislav (modified for Rust code generation)
 */
public class EntryPoint {

  public static void main(String[] args) {
    boolean status = new CommandDispatcher(args, new Command[] {
      new BuildMetadataRustFromXml()
    }).start();

    System.exit(status ? 0 : 1);
  }
}

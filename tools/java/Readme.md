## This directory contains script for autogeneration of metadata in rust

To build from source cd /tools/java and 
```
mvn install
```

Example command on build generator 
```
java -jar tools\java\rust-build\target\rust-build-1.0-SNAPSHOT-jar-with-dependencies.jar BuildMetadataRustFromXml resources\PhoneNumberMetadata.xml ./test.rs metadata --const-name=test
```
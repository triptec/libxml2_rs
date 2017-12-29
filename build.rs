extern crate bindgen;

fn main() {
  // Tell cargo to tell rustc to link the system xml2
  // shared library.
  println!("cargo:rustc-link-lib=xml2");

  // The bindgen::Builder is the main entry point
  // to bindgen, and lets you build up options for
  // the resulting bindings.
  let bindings = bindgen::Builder::default()
      // The input header we would like to generate
      // bindings for.
      .header("src/libxml2/wrapper.h")
      //.whitelist_recursively(false)

      /*
      .whitelist_type("xmlDocPtr")
      .whitelist_type("xmlDoc")
      .whitelist_type("_xmlDoc")

      .whitelist_type("xmlElementType")

      .whitelist_type("_xmlNode")
      */
      .whitelist_function("xmlNewDoc")
      .whitelist_function("xmlFreeDoc")
      .whitelist_function("xmlDocGetRootElement")
      .whitelist_function("xmlReadMemory")
      .whitelist_function("xmlReadFile")
      .whitelist_function("xmlResetLastError")
      .whitelist_function("xmlSetStructuredErrorFunc")
      .whitelist_function("xmlNodeSetName")
      .whitelist_function("xmlDocDumpMemoryEnc")
      .whitelist_function("xmlDocDumpFormatMemoryEnc")
      .whitelist_function("xmlBufNodeDump")
      .whitelist_function("xmlNodeDump")
      .whitelist_function("xmlBufferContent")
      .whitelist_function("xmlBufferCreate")
      .whitelist_function("xmlBufferFree")
      .whitelist_function("xmlResetError")
      .whitelist_function("xmlDocCopyNode")
      .whitelist_function("xmlAddChild")
      .whitelist_function("xmlUnlinkNode")
      .whitelist_function("xmlFreeNode")
      .whitelist_function("xmlSaveFile")
      .whitelist_function("htmlReadFile")
      .whitelist_function("xmlGetLastChild")
      .whitelist_function("xmlNewDocNode")
      .whitelist_function("xmlDocSetRootElement")
      .whitelist_function("xmlAddPrevSibling")
      .whitelist_function("xmlGetProp")
      .whitelist_function("xmlHasProp")
      .whitelist_function("xmlSetProp")
      .whitelist_function("xmlRemoveProp")
      .whitelist_function("xmlNodeGetContent")
      .whitelist_function("xmlNodeAddContentLen")

      // Homebrew location of libxml2 headers.
      .clang_arg("-I/usr/include/libxml2")
      // Finish the builder and generate the bindings.
      .generate()
      // Unwrap the Result and panic on failure.
      .expect("Unable to generate bindings");

  bindings
      .write_to_file("src/libxml2/mod.rs")
      .expect("Couldn't write bindings!");
}

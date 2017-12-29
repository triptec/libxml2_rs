#![feature(pointer_methods)]

#[macro_use]
extern crate bitflags;
extern crate libc;

#[allow(dead_code, non_camel_case_types, non_upper_case_globals, non_snake_case)]
mod libxml2;

mod tree;
pub use tree::ParseOptions;

use tree::{XmlError, XmlInput};
pub use tree::document::Document;
pub use tree::node::{Node, NodeType};

pub fn xml_with_options<R: XmlInput + ?Sized>(r:&R, url: &str, encoding: &str, options: ParseOptions) -> Result<Document, Vec<XmlError>> {
    Document::parse_with_options(r, url, encoding, options)
}

pub fn xml<R: XmlInput + ?Sized>(r:&R) -> Result<Document, Vec<XmlError>> {
    Document::parse(r)
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::path::Path;
    use super::*;
    #[test]
    fn lib_test(){
        assert!(xml("<root></root>").is_ok());
        assert!(xml(&String::from("<root></root>")).is_ok());
        assert!(xml(&File::open("tests/resources/file01.xml").unwrap()).is_ok());
        assert!(xml(Path::new("tests/resources/file01.xml")).is_ok());
    }
}

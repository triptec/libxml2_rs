use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::ffi::{ CString, CStr };
use std::mem;
use std::ptr;
use std::str;
use std::os::raw::{c_void, c_int};

use libxml2::{xmlBufferCreate,
              xmlBufferContent,
              xmlFreeDoc,
              xmlBufferFree,
              xmlReadMemory,
              xmlReadFile,
              xmlResetLastError,
              xmlSetStructuredErrorFunc,
              xmlDocGetRootElement,
              xmlDocSetRootElement,
              xmlDocDumpMemoryEnc,
              xmlDocDumpFormatMemoryEnc,
              xmlNodeDump,
              xmlDocCopyNode,
              xmlUnlinkNode,
              xmlSaveFile,
              xmlNewDoc,
              xmlDocPtr,
              xmlNodePtr};

use tree::{ParseOptions, XmlInput, XmlError, error_vec_pusher};
use tree::node::{Node};

pub type DocumentRef = Rc<RefCell<_Document>>;

#[derive(Debug)]
pub struct _Document {
    // TODO: How to make public only in this package?
    pub doc_ptr: xmlDocPtr,
    errors: Vec<XmlError>,
    pub nodes: HashMap<xmlNodePtr, Node>,
}

impl _Document {
    pub fn insert_node(&mut self, node_ptr: xmlNodePtr, node: Node) {
        // TODO: check that _Node.document is self
        self.nodes.insert(node_ptr, node);
    }
}

#[derive(Clone)]
pub struct Document(DocumentRef);

impl Drop for _Document {
    ///Free document when it goes out of scope
    fn drop(&mut self) {
        let doc_ptr = self.doc_ptr;
        unsafe {
            xmlFreeDoc(doc_ptr);
        }
    }
}

impl Document {

    /// Creates a new empty libxml2 document
    pub fn new() -> Result<Self, ()> {
        unsafe {
            let c_version = CString::new("1.0").unwrap();
            let doc_ptr = xmlNewDoc(c_version.as_ptr() as *const u8);
            if doc_ptr.is_null() {
                Err(())
            } else {
                let doc = _Document{doc_ptr: doc_ptr, errors: vec![], nodes: HashMap::new()};
                Ok(Document(Rc::new(RefCell::new(doc))))
            }
        }
    }

    pub fn doc_ref(&self) -> DocumentRef {
        self.0.clone()
    }

    pub fn doc_ptr(&self) -> xmlDocPtr {
        self.0.borrow().doc_ptr
    }

    fn ptr_as_option(&mut self, node_ptr: xmlNodePtr) -> Option<Node> {
        if node_ptr.is_null() {
            None
        } else {
            let new_node = Node::wrap(node_ptr, self.0.clone());
            self.0.borrow_mut().insert_node(node_ptr, new_node.clone());
            Some(new_node)
        }
    }

    /// Import a `Node` from another `Document`
    pub fn import_node(&mut self, node: &mut Node) -> Option<Node> {
        let new_node_ptr = unsafe {
            xmlDocCopyNode(node.node_ptr(), self.doc_ptr(), 1)
        };
        self.ptr_as_option(new_node_ptr)
    }


    /// Get the root element of the document
    pub fn get_root_element(&self) -> Option<Node> {
        unsafe {
            let node_ptr = xmlDocGetRootElement(self.0.borrow().doc_ptr);
            if node_ptr.is_null() {
                None
            } else {
                let node = Node::wrap(node_ptr, self.0.clone());
                self.0.borrow_mut().nodes.insert(node_ptr, node.clone());
                Some(node)
            }
        }
    }

    /// Sets the root element of the document
    pub fn set_root_element(&mut self, root: &mut Node) {
        unsafe {
            // TODO: returns old root if any, should we do something with that?
            xmlDocSetRootElement(self.doc_ptr(), root.node_ptr());
        }
    }

    pub fn to_string(&self, format: bool) -> String {
        unsafe {
            // allocate a buffer to dump into
            //let mut receiver = ptr::null_mut();
            let mut receiver = ptr::null_mut();
            let mut size: c_int = 0;
            let c_utf8 = CString::new("UTF-8").unwrap();
            let doc_ptr = self.0.borrow().doc_ptr;
            if !format {
                xmlDocDumpMemoryEnc(doc_ptr, &mut receiver, &mut size, c_utf8.as_ptr());
            } else {
                xmlDocDumpFormatMemoryEnc(doc_ptr, &mut receiver, &mut size, c_utf8.as_ptr(), 1);
            }

            let c_string = CStr::from_ptr(receiver as *const i8);
            let node_string = str::from_utf8(c_string.to_bytes()).unwrap().to_owned();
            mem::forget(receiver);
            node_string
        }
    }

    /// Write document to `filename`
    pub fn save_file(&self, filename: &str) -> Result<c_int, ()> {
        let c_filename = CString::new(filename).unwrap();
        unsafe {
            let retval = xmlSaveFile(c_filename.as_ptr(), self.doc_ptr());
            if retval < 0 {
                return Err(());
            }
            Ok(retval)
        }
    }

    pub fn parse<R: XmlInput + ?Sized>(r:&R) -> Result<Document, Vec<XmlError>> {
        Document::parse_with_options(r, "", "utf-8", ParseOptions::DEFAULT_XML)
    }

    pub fn parse_with_options<R: XmlInput + ?Sized>(r:&R, url: &str, encoding: &str, options: ParseOptions) -> Result<Document, Vec<XmlError>> {
        match r.is_path() {
            true => Document::parse_file(&r.data(), encoding, options),
            false => Document::parse_string(&r.data(), url, encoding, options)
        }
    }

    fn parse_string(xml_str: &str, url: &str, encoding: &str, options: ParseOptions) -> Result<Document, Vec<XmlError>> {
        let c_string_len = xml_str.len() as i32;
        let c_string = CString::new(xml_str).unwrap();
        let c_utf8 = CString::new(encoding).unwrap();
        let c_url = CString::new(url).unwrap();
        Document::parse_handler(|| unsafe { xmlReadMemory(c_string.as_ptr(), c_string_len, c_url.as_ptr(), c_utf8.as_ptr(), options.bits as i32) })
    }

    fn parse_file(filename: &str, encoding: &str, options: ParseOptions) -> Result<Document, Vec<XmlError>> {
        let c_filename = CString::new(filename).unwrap();
        let c_utf8 = CString::new(encoding).unwrap();

        Document::parse_handler(|| unsafe { xmlReadFile(c_filename.as_ptr(), c_utf8.as_ptr(), options.bits as i32) })
    }

    fn parse_handler<F>(parse_closure: F) -> Result<Document, Vec<XmlError>> where F: Fn() -> xmlDocPtr {
        unsafe {
            let errors: Box<Vec<XmlError>> = Box::new(vec![]);
            xmlResetLastError();
            let errors_ptr: *mut c_void = mem::transmute(errors);
            xmlSetStructuredErrorFunc(errors_ptr, Some(error_vec_pusher));
            let doc_ptr = parse_closure();
            xmlSetStructuredErrorFunc(ptr::null_mut(), None);
            Document::handle_result_ptrs(doc_ptr, errors_ptr)
        }
    }

    fn handle_result_ptrs(doc_ptr: xmlDocPtr, errors_ptr: *mut c_void) -> Result<Document, Vec<XmlError>> {
        let errors: Box<Vec<XmlError>> = unsafe { mem::transmute(errors_ptr) };
        match doc_ptr.is_null() {
            true => {
                unsafe { xmlFreeDoc(doc_ptr) };

                // Nokogiri raises the last error, not sure what we want or what would be idiomatic.
                //Err(xml_get_last_error())

                Err(*errors)
            }
            false => {
                // TODO: Implement XInclude
                let doc = _Document{doc_ptr: doc_ptr, errors: *errors, nodes: HashMap::new()};
                Ok(Document(Rc::new(RefCell::new(doc))))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_string_test(){
        assert_eq!(true, Document::parse_string("<root></root>", "", "utf-8", ParseOptions::DEFAULT_XML).is_ok());
        assert_eq!(true, Document::parse_string("a><root></root>", "", "utf-8", ParseOptions::DEFAULT_XML).is_ok());
    }

    #[test]
    fn get_root_element_test(){
        let doc = Document::parse("<root></root>").unwrap();
        let node = doc.get_root_element().unwrap();
    }

    #[test]
    fn document_can_import_node() {
        let xml_string = String::from(r#"<root>
            <child attribute="value">some text</child>
            <child attribute="empty">more text</child>
        </root>"#);
        let doc1 = Document::parse(&xml_string).unwrap();
        let mut doc2 = Document::parse(&xml_string).unwrap();

        assert_eq!(doc2.get_root_element().unwrap().get_child_elements().len(), 2);

        let elements = doc1.get_root_element().unwrap().get_child_elements();
        let node = elements.first().unwrap();
        let imported = doc2.import_node(&mut node.clone()).unwrap();
        assert!(doc2.get_root_element().unwrap().add_child(imported).is_ok());
        assert_eq!(doc2.get_root_element().unwrap().get_child_elements().len(), 3);
    }
}

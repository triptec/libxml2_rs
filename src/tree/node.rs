use std::cell::RefCell;
use std::rc::Rc;
use std::ffi::{CStr, CString};
use std::str;
use libxml2::{xmlNodePtr,
              xmlBufferCreate,
              xmlBufferContent,
              xmlBufferFree,
              xmlNodeDump,
              xmlNodeSetName};

use tree::document::DocumentRef;

pub type NodeRef = Rc<RefCell<_Node>>;

#[derive(Debug)]
pub struct _Node {
    node_ptr: xmlNodePtr,
    document: DocumentRef,
}

#[derive(Debug, Clone)]
pub struct Node(NodeRef);

impl Node {
    pub fn new(node_ptr: xmlNodePtr, document: DocumentRef) -> Node {
        let node = _Node { node_ptr, document };
        Node(Rc::new(RefCell::new(node)))
    }


    /// Returns all child nodes of the given node as a vector
    pub fn get_child_nodes(&self) -> Vec<Node> {
        let mut nodes = Vec::new();
        if let Some(node) = self.get_first_child() {
            nodes.push(node.clone());
            let mut current_node = node;
            while let Some(sibling) = current_node.get_next_sibling() {
                current_node = sibling.clone();
                nodes.push(sibling)
            }
        }
        nodes
    }

    /// Returns the first child if it exists
    pub fn get_first_child(&self) -> Option<Node> {
        let node_ptr = self.0.borrow().node_ptr;
        let first_child_ptr = unsafe { (*node_ptr).children };
        Node::ptr_as_option(self, first_child_ptr)
    }

    /// Returns the next sibling if it exists
    pub fn get_next_sibling(&self) -> Option<Node> {
        let node_ptr = self.0.borrow().node_ptr;
        let next_sibling_ptr = unsafe { (*node_ptr).next };
        Node::ptr_as_option(self, next_sibling_ptr)
    }

    /// Returns the name of the node (empty string if name pointer is `NULL`)
    pub fn get_name(&self) -> String {
        let node_ptr = self.0.borrow().node_ptr;
        let name_ptr = unsafe { (*node_ptr).name as *const i8 };
        if name_ptr.is_null() {
            return String::new();
        }  //empty string
        let c_string = unsafe { CStr::from_ptr(name_ptr) };
        str::from_utf8(c_string.to_bytes()).unwrap().to_owned()
    }

    /// Sets the name of this `Node`
    pub fn set_name(&mut self, name: &str) {
        let c_name = CString::new(name).unwrap();
        let node_ptr = self.0.borrow_mut().node_ptr;
        unsafe { xmlNodeSetName(node_ptr, c_name.as_ptr() as *const u8) }
    }

    /// Serializes a `Node`
    pub fn to_string(&self, format: bool) -> String {
        let format = if format {
            1
        } else {
            0
        };
        let doc = &self.0.borrow().document;
        let doc_ptr = doc.borrow().doc_ptr;
        let node_ptr = self.0.borrow().node_ptr;
        unsafe {
            // allocate a buffer to dump into
            let buf = xmlBufferCreate();

            // dump the node
            xmlNodeDump(buf,
                        doc_ptr,
                        node_ptr,
                        1, // level of indentation
                        format /* disable formatting */);
            let result_ptr = xmlBufferContent(buf);
            let c_string = CStr::from_ptr(result_ptr as *const i8);
            let node_string = str::from_utf8(c_string.to_bytes()).unwrap().to_owned();
            xmlBufferFree(buf);

            node_string
        }
    }
    fn ptr_as_option(node: &Node, node_ptr: xmlNodePtr) -> Option<Node> {
        if node_ptr.is_null() {
            None
        } else {
            let new_node = Node::new(node_ptr, node.0.borrow().document.clone());
            let doc = &node.0.borrow().document;
            doc.borrow_mut().insert_node(node_ptr, new_node.clone());
            Some(new_node)
        }
    }

}

#[cfg(test)]
mod tests {
    use tree::document::Document;

    #[test]
    fn get_first_child_next_sibling_test() {
        let doc = Document::parse("<root><child></child><sibling></sibling></root>").unwrap();
        let node = doc.get_root_element().unwrap();
        let child = node.get_first_child();
        assert!(child.is_some());
        let sibling = child.unwrap().get_next_sibling();
        assert!(sibling.is_some());
    }

    #[test]
    fn get_child_nodes_test() {
        let doc = Document::parse("<root><child></child><sibling></sibling></root>").unwrap();
        let node = doc.get_root_element().unwrap();
        let child_nodes = node.get_child_nodes();
        assert_eq!(2, child_nodes.len());
    }

    #[test]
    fn set_name_test() {
        let doc = Document::parse("<root><child>child</child><sibling>sibling</sibling></root>").unwrap();
        let node = doc.get_root_element().unwrap();
        let child_nodes = node.get_child_nodes();
        assert_eq!(2, child_nodes.len());
        for mut child_node in child_nodes {
            child_node.set_name("lol");
        }
        assert_eq!("<root><lol>child</lol><lol>sibling</lol></root>", node.to_string(false));
    }

}
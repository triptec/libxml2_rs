use std::cell::RefCell;
use std::rc::Rc;
use std::ffi::{CStr, CString};
use std::os::raw::{c_uint};
use std::ptr;
use std::str;
use std::mem;
use std::collections::HashMap;

use libc;

use libxml2::{xmlNodePtr,
              xmlNsPtr,
              xmlBufferCreate,
              xmlBufferContent,
              xmlBufferFree,
              xmlNodeDump,
              xmlAddChild,
              xmlAddPrevSibling,
              xmlGetLastChild,
              xmlUnlinkNode,
              xmlFreeNode,
              xmlNewDocNode,
              xmlGetProp,
              xmlHasProp,
              xmlSetProp,
              xmlRemoveProp,
              xmlNodeGetContent,
              xmlNodeAddContentLen,
              xmlNodeSetName};

use tree::document::DocumentRef;

pub type NodeRef = Rc<RefCell<_Node>>;

#[derive(Debug)]
pub struct _Node {
    node_ptr: xmlNodePtr,
    document: DocumentRef,
    unlinked: bool,
}

#[derive(Debug, Clone)]
pub struct Node(NodeRef);


/// Types of xml nodes
#[derive(Debug, PartialEq)]
pub enum NodeType {
    ElementNode,
    AttributeNode,
    TextNode,
    CDataSectionNode,
    EntityRefNode,
    EntityNode,
    PiNode,
    CommentNode,
    DocumentNode,
    DocumentTypeNode,
    DocumentFragNode,
    NotationNode,
    HtmlDocumentNode,
    DTDNode,
    ElementDecl,
    AttributeDecl,
    EntityDecl,
    NamespaceDecl,
    XIncludeStart,
    XIncludeEnd,
    DOCBDocumentNode,
}

impl NodeType {
    /// converts an integer from libxml's `enum NodeType`
    /// to an instance of our `NodeType`
    pub fn from_c_int(i: c_uint) -> Option<NodeType> {
        match i {
            1 => Some(NodeType::ElementNode),
            2 => Some(NodeType::AttributeNode),
            3 => Some(NodeType::TextNode),
            4 => Some(NodeType::CDataSectionNode),
            5 => Some(NodeType::EntityRefNode),
            6 => Some(NodeType::EntityNode),
            7 => Some(NodeType::PiNode),
            8 => Some(NodeType::CommentNode),
            9 => Some(NodeType::DocumentNode),
            10 => Some(NodeType::DocumentTypeNode),
            11 => Some(NodeType::DocumentFragNode),
            12 => Some(NodeType::NotationNode),
            13 => Some(NodeType::HtmlDocumentNode),
            14 => Some(NodeType::DTDNode),
            15 => Some(NodeType::ElementDecl),
            16 => Some(NodeType::AttributeDecl),
            17 => Some(NodeType::EntityDecl),
            18 => Some(NodeType::NamespaceDecl),
            19 => Some(NodeType::XIncludeStart),
            20 => Some(NodeType::XIncludeEnd),
            21 => Some(NodeType::DOCBDocumentNode),
            _ => None,
        }
    }
}

impl Drop for _Node {
    fn drop(&mut self) {
        //println!("_Node::drop");
        if self.unlinked {
            unsafe { xmlFreeNode(self.node_ptr) }
        }
    }
}
/*
impl Drop for Node {
    fn drop(&mut self) {
        println!("Node::drop");
        println!("Before: Strong count: {}, Weak count: {}", Rc::strong_count(&self.0), Rc::weak_count(&self.0));

        {
            let inner_node = self.0.borrow();
            println!("unlinked: {}", inner_node.unlinked);
            if !inner_node.unlinked {
                return;
            }

            let mut doc = inner_node.document.try_borrow_mut();
            if doc.is_err() {
                return;
            }
            let mut inner_doc = doc.unwrap();
            if !inner_doc.nodes.contains_key(&self.node_ptr()) {
                return;
            }

            println!("Removing node {:?} from nodes", &self.node_ptr());
            inner_doc.nodes.remove(&self.node_ptr());
        }
        println!("After: Strong count: {}, Weak count: {}", Rc::strong_count(&self.0), Rc::weak_count(&self.0));

        /*
        if (Rc::strong_count(&self.0)) <= 1 {
            xmlFreeNode(&self.node_ptr());
        }
        */
    }
}
*/

impl PartialEq for Node {
    /// Two nodes are considered equal, if they point to the same xmlNode.
    fn eq(&self, other: &Node) -> bool {
        self.node_ptr() == other.node_ptr()
    }
}

impl Eq for Node {}

impl Node {
    pub fn node_ptr(&self) -> xmlNodePtr {
        self.0.borrow().node_ptr
    }

    pub fn node_ptr_mut(&mut self) -> xmlNodePtr {
        self.0.borrow_mut().node_ptr
    }

    pub fn wrap(node_ptr: xmlNodePtr, document: DocumentRef) -> Node {
        let node = _Node { node_ptr, document, unlinked: false };
        Node(Rc::new(RefCell::new(node)))
    }

    /// Create a new node, bound to a given document.
    pub fn new(name: &str, ns: Option<xmlNsPtr>, document: DocumentRef) -> Result<Self, ()> {
        // We will only allow to work with document-bound nodes for now, to avoid the problems of memory management.

        let c_name = CString::new(name).unwrap();
        let ns_ptr = match ns {
            None => ptr::null_mut(),
            Some(ns) => ns,
        };
        let node_ptr = unsafe { xmlNewDocNode(document.borrow().doc_ptr, ns_ptr, c_name.as_ptr() as *const u8, ptr::null()) };
        if node_ptr.is_null() {
            Err(())
        } else {
            let new_node = Node::wrap(node_ptr, document.clone());
            document.borrow_mut().insert_node(node_ptr, new_node.clone());
            Ok(new_node)
        }
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

    /// Returns all child elements of the given node as a vector
    pub fn get_child_elements(&self) -> Vec<Node> {
        self.get_child_nodes().into_iter().filter(
            |n| n.get_type() == Some(NodeType::ElementNode)
        ).collect::<Vec<Node>>()
    }

    /// Get the node type
    pub fn get_type(&self) -> Option<NodeType> {
        NodeType::from_c_int(unsafe { (*self.node_ptr()).type_ })
    }

    /// Returns true iff it is a text node
    pub fn is_text_node(&self) -> bool {
        match self.get_type() {
            Some(NodeType::TextNode) => true,
            _ => false,
        }
    }

    /// Returns the first child if it exists
    pub fn get_first_child(&self) -> Option<Node> {
        let first_child_ptr = unsafe { (*self.node_ptr()).children };
        Node::ptr_as_option(self, first_child_ptr)
    }

    pub fn get_last_child(&self) -> Option<Node> {
        let last_child_ptr = unsafe { xmlGetLastChild(self.node_ptr()) };
        Node::ptr_as_option(self, last_child_ptr)
    }

    /// Returns the next sibling if it exists
    pub fn get_next_sibling(&self) -> Option<Node> {
        let node_ptr = self.0.borrow().node_ptr;
        let next_sibling_ptr = unsafe { (*node_ptr).next };
        Node::ptr_as_option(self, next_sibling_ptr)
    }

    /// Creates a new `Node` as child to the self `Node`
    pub fn add_child(&mut self, child: &mut Node) -> Result<Node, ()> {
        let node_ptr = unsafe { xmlAddChild(self.node_ptr(), child.node_ptr()) };
        Node::ptr_as_result(self, node_ptr)
    }

    /// Add a previous sibling
    pub fn add_prev_sibling(&mut self, new_sibling: Node) -> Option<Node> {
        // TODO: Think of using a Result type, the libxml2 call returns NULL on error, or the child node on success
        unsafe {
            if xmlAddPrevSibling(self.node_ptr(), new_sibling.node_ptr()).is_null() {
                None
            } else {
                Some(new_sibling)
            }
        }
    }

    /// Unbinds the Node from its siblings and Parent, but not from the Document it belongs to.
    /// If the node is not inserted into the DOM afterwards, it will be lost after the program terminates.
    /// From a low level view, the unbound node is stripped from the context it is and inserted into a (hidden) document-fragment.
    pub fn unlink(&mut self) {
        let node_type = self.get_type();
        if node_type != Some(NodeType::DocumentNode) && node_type != Some(NodeType::DocumentFragNode) && !self.0.borrow().unlinked {
            unsafe {
                self.0.borrow_mut().unlinked = true;
                let doc = &self.0.borrow().document;
                doc.borrow_mut().nodes.remove(&self.node_ptr());
                xmlUnlinkNode( self.node_ptr() );
            }
        }
    }

    /// Returns the content of the node
    /// (empty string if content pointer is `NULL`)
    pub fn get_content(&self) -> String {
        let content_ptr = unsafe { xmlNodeGetContent(self.node_ptr()) };
        if content_ptr.is_null() {
            return String::new();
        }  //empty string
        let c_string = unsafe { CStr::from_ptr(content_ptr as *const i8) };
        str::from_utf8(c_string.to_bytes()).unwrap().to_owned()
    }

    /// Append text to this `Node`
    pub fn append_text(&mut self, content: &str) {
        let c_len = content.len() as i32;
        if c_len > 0 {
            let c_content = CString::new(content).unwrap();
            unsafe {
                xmlNodeAddContentLen(self.node_ptr(), c_content.as_ptr() as *const u8, c_len);
            }
        }
    }

    /// Returns the name of the node (empty string if name pointer is `NULL`)
    pub fn get_name(&self) -> String {
        let name_ptr = unsafe { (*self.node_ptr()).name as *const i8 };
        if name_ptr.is_null() {
            return String::new();
        }  //empty string
        let c_string = unsafe { CStr::from_ptr(name_ptr) };
        str::from_utf8(c_string.to_bytes()).unwrap().to_owned()
    }

    /// Sets the name of this `Node`
    pub fn set_name(&mut self, name: &str) {
        let c_name = CString::new(name).unwrap();
        unsafe { xmlNodeSetName(self.node_ptr_mut(), c_name.as_ptr() as *const u8) }
    }

    /// Get a copy of the attributes of this node
    pub fn get_properties(&self) -> HashMap<String, String> {
        let mut attributes = HashMap::new();
        let mut attr_names = Vec::new();
        unsafe {
            let mut current_prop = (*self.node_ptr()).properties;
            while !current_prop.is_null() {
                let name_ptr = (*current_prop).name;
                let c_name_string = CStr::from_ptr(name_ptr as *const i8);
                let name = str::from_utf8(c_name_string.to_bytes()).unwrap().to_owned();
                attr_names.push(name);
                current_prop = (*current_prop).next;
            }
        }

        for name in attr_names {
            let value = self.get_property(&name).unwrap_or(String::new());
            attributes.insert(name, value);
        }

        attributes
    }

    /// Returns the value of property `name`
    pub fn get_property(&self, name: &str) -> Option<String> {
        let c_name = CString::new(name).unwrap();
        let value_ptr = unsafe { xmlGetProp(self.node_ptr(), c_name.as_ptr() as *const u8) };
        if value_ptr.is_null() {
            return None;
        }
        let c_value_string = unsafe { CStr::from_ptr(value_ptr as *const i8) };
        let prop_str = str::from_utf8(c_value_string.to_bytes()).unwrap().to_owned();
        unsafe {
            libc::free(value_ptr as *mut libc::c_void);
        }
        Some(prop_str)
    }

    /// Alias for get_property
    pub fn get_attribute(&self, name: &str) -> Option<String> {
        self.get_property(name)
    }

    /// Return an attribute as a `Node` struct of type AttributeNode
    pub fn get_property_node(&self, name: &str) -> Option<Node> {
        let c_name = CString::new(name).unwrap();
        unsafe {
            let attr_node = xmlHasProp(self.node_ptr(), c_name.as_ptr() as *const u8);
            Self::ptr_as_option(self, attr_node as xmlNodePtr)
        }
    }

    /// Alias for get_property_node
    pub fn get_attribute_node(&self, name: &str) -> Option<Node> {
        self.get_property_node(name)

    }

    /// Alias for `get_properties`
    pub fn get_attributes(&self) -> HashMap<String, String> {
        self.get_properties()
    }

    /// Sets the value of property `name` to `value`
    pub fn set_property(&mut self, name: &str, value: &str) {
        let c_name = CString::new(name).unwrap();
        let c_value = CString::new(value).unwrap();
        unsafe { xmlSetProp(self.node_ptr(), c_name.as_ptr() as *const u8, c_value.as_ptr() as *const u8) };
    }

    /// Alias for set_property
    pub fn set_attribute(&mut self, name: &str, value: &str) {
        self.set_property(name, value)
    }

    /// Removes the property of given `name`
    pub fn remove_property(&mut self, name: &str) {
        // TODO: Should we make the API return a Result type here?
        // Current behaviour on failures: silently return (noop)
        let c_name = CString::new(name).unwrap();
        unsafe {
            let attr_node = xmlHasProp(self.node_ptr(), c_name.as_ptr() as *const u8);
            if !attr_node.is_null() {
                xmlRemoveProp(attr_node);
            }
        }
    }

    /// Alias for remove_property
    pub fn remove_attribute(&mut self, name: &str) {
        self.remove_property(name)
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
        unsafe {
            // allocate a buffer to dump into
            let buf = xmlBufferCreate();

            // dump the node
            xmlNodeDump(buf,
                        doc_ptr,
                        self.node_ptr(),
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
            let new_node = Node::wrap(node_ptr, node.0.borrow().document.clone());
            let doc = &node.0.borrow().document;
            doc.borrow_mut().insert_node(node_ptr, new_node.clone());
            Some(new_node)
        }
    }

    fn ptr_as_result(node: &Node, node_ptr: xmlNodePtr) -> Result<Node, ()> {
        if node_ptr.is_null() {
            Err(())
        } else {
            let new_node = Node::wrap(node_ptr, node.0.borrow().document.clone());
            let doc = &node.0.borrow().document;
            doc.borrow_mut().insert_node(node_ptr, new_node.clone());
            Ok(new_node)
        }
    }
}

#[cfg(test)]
mod tests {
    use tree::document::Document;
    use std::rc::Rc;

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

    #[test]
    fn unlink_test() {
        let doc = Document::parse("<root><child>child</child><sibling>sibling</sibling></root>").unwrap();
        {
            let mut node = doc.get_root_element().unwrap();
            node.unlink();
        }
    }
}
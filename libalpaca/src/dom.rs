//! Contains parsing routines
use aux;
use html5ever::{ interface::QualName, LocalName, ns, namespace_url, serialize, serialize::{SerializeOpts} };
use kuchiki::traits::*;
use kuchiki::{ parse_html_with_options, NodeRef, ParseOpts };
use std::fs::File;
use std::io::prelude::*;
use std::{ str, fs, path::Path };
use std::ffi::CStr;


/// Defines our basic object types, each of which has a corresponding
/// unique (distribution, padding type) tuple.
#[derive(PartialEq)]
pub enum ObjectKind {
    FakeIMG,  /// Fake alpaca image
    HTML   ,
    CSS    ,
    IMG    ,  /// IMG: PNG, JPEG, etc.
	JS     ,
	CssImg ,
    Unknown,
}

/// An object to be used in the morphing process.
pub struct Object {
    /// Type of the Object
    pub kind: ObjectKind,
    /// Content (Vector of bytes) of the Object
    pub content: Vec<u8>,
    /// Node in the html
    pub node: Option<NodeRef>,
    /// Size to pad the Object to
    pub target_size: Option<usize>,
    /// The uri of the object, as mentioned in the html source
    pub uri: String,
}

#[repr(C)]
pub struct map {
    pub elems: *mut *mut cell,
    pub capacity: libc::c_int,
    pub size: libc::c_int,
}

#[repr(C)]
pub struct cell {
    pub next: *mut cell,
    pub value: *mut libc::c_void,
    pub key: [libc::c_char; 0],
}

pub type Map = *mut map;

#[link(name = "map", kind = "static")]

extern "C" {
    fn map_create() -> Map;
    fn map_set(m: Map, key: *const libc::c_char, value: *mut libc::c_void);
    fn map_get(m: Map, key: *const libc::c_char) -> *mut libc::c_void;
}

impl Object {

    /// Construct a real object from the html page
    pub fn existing(content: &[u8], kind: ObjectKind, uri: String, node: &NodeRef) -> Object {
        Object {
            kind        : kind               ,
            content     : content.to_vec()   ,
            node        : Some( node.clone() ),
            target_size : None               ,
            uri         : uri                ,
        }
    }

    /// Create padding object
    pub fn fake_image(target_size: usize) -> Object {
        Object {
            kind        : ObjectKind::FakeIMG       ,
            content     : Vec::new()                ,
            node        : None                      ,
            target_size : Some(target_size)         ,
            uri         : String::from("pad_object"),
        }
    }
}

pub fn create_css_node(css_text: &str) -> NodeRef {

	let elem_node = create_element("style");
	let css_text  = NodeRef::new_text(css_text);

    elem_node.append(css_text);
	elem_node
}

pub fn css_parse_all_images(css_text: &str) -> Vec<String>{

	let mut images_paths: Vec<String> = Vec::new();

	if css_text.contains("url"){

		let spl_val: Vec<&str> = css_text.split("\n").collect();

		for item in spl_val {

			let mut new_it = remove_whitespace(&item);
			// println!("{}", new_it);

			if new_it.contains("url") {

                new_it = new_it.replace("\'", "\"");

                let spl       = new_it.split("url");
				let mut found = false;

                for it in spl {
					// println!("{}" , it);
					if found == true {
						let path = it.replace("\"", "").replace("(", "").replace(")", "").replace(")", "").replace(";", "");
						images_paths.push(path);
						break;
					}
					found = true;
				}
			}
		}
	}
	return images_paths;
}

pub fn copy_file_to_string(fname : &str) -> Result<String, std::io::Error> {

    // Create a path to the desired file
    let path    = Path::new(fname);
    let display = path.display();

    // println!("{}",display);

    // Open the path in read-only mode, returns `io::Result<File>`
    let mut file = match File::open(&path){

        Err(why) => {
            println!("couldn't open {}: {}", display, why);
            return Err(why)
        },

        Ok(file) => {
            println!("OPENED");
            file
        },
    };

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut s = String::new();

    match file.read_to_string(&mut s) {
        Err(why) => panic!("couldn't read {}: {}", display, why),
        Ok(_)    => ()
    }

    // `file` goes out of scope, and the "hello.txt" file gets closed
    Ok(s)
}

fn remove_whitespace(s: &str) -> String {
    s.chars().filter( |c| !c.is_whitespace() ).collect()
}

/// Parses the object's kind from its raw representation
pub fn parse_object_kind(mime: &str) -> ObjectKind {
	match mime {
		"text/html"                  => ObjectKind::HTML,
		"text/css"                   => ObjectKind::CSS ,
		x if x.starts_with("image/") => ObjectKind::IMG ,
    	_                            => ObjectKind::Unknown
    }
}

/// Parses the target size of an object from its HTTP request query.
/// Returns 0 on error.
pub fn parse_target_size(query: &str) -> usize {

    let split1: Vec<&str> = query.split("alpaca-padding=").collect();
	let split2: Vec<&str> = split1[ split1.len() - 1 ].split("&").collect();
    let size_str          = split2[0];

	// Return the size
	match size_str.parse::<usize>() {
	  Ok(size) => return size,
	  Err(_)   => return 0
	}
}

/// Parses the objects contained in an HTML page.
pub fn parse_object_names(document: &NodeRef) -> Vec<String> {

    // Objects vector
	let mut objects: Vec<String> = Vec::new();
	let mut found_favicon        = false;

    for node_data in document.select("img,link,script").unwrap() {

        let node = node_data.as_node();
		let name = node_data.name.local.to_lowercase();


		let path_attr = if name == "link" { "href" } else { "src" };
		let path      = match node_get_attribute(node, path_attr) {
			Some(p) if p != "" && !p.starts_with("data:") => p       ,
			_                                             => continue,
		};

		println!("PATH {}",path);

		let temp = format!( "/{}", path.as_str());

		objects.push(temp);

		let rel  = node_get_attribute(node, "rel").unwrap_or_default();
		match ( name.as_str(), rel.as_str() ) {
			("link", "stylesheet")                       => ObjectKind::CSS                          ,
			("link", "shortcut icon") | ("link", "icon") => { found_favicon = true; ObjectKind::IMG },
			("script", _)                                => ObjectKind::JS                           ,
			("img", _)                                   => ObjectKind::IMG                          ,
			_                                            => continue                                 ,
		};
	}

	// If no favicon was found, insert an empty one
	if !found_favicon {
		insert_empty_favicon(document);
	}

    // objects.sort_unstable_by( |a, b| b.content.len().cmp( &a.content.len() ) ); // larger first
	objects
}

pub fn get_map_element(req_mapper : Map , uri : String) -> String {

	let c_uri: *mut libc::c_char = uri.as_ptr() as *mut libc::c_char;
	println!("{}",uri);
    let temp = unsafe { map_get(req_mapper, c_uri) } as *mut libc::c_char;
    let temp = unsafe { CStr::from_ptr(temp) };
    let str_slice: &str = temp.to_str().unwrap();
    println!("{}",str_slice);

	str_slice.to_owned()
}



pub fn parse_objects_from_content(document: &NodeRef, req_mapper : Map , alias: usize) -> () {

	let mut objects: Vec<Object> = Vec::with_capacity(10);
	let mut found_favicon        = false;

	for node_data in document.select("link").unwrap() {
		let node      = node_data.as_node();
		let path_attr = "href";

        let path = match node_get_attribute(node, path_attr) {
			Some(p) if p != "" && !p.starts_with("data:") => p       ,
			_                                             => continue,
		};

		if path.contains("favicon.ico") {
			continue;
		}

		// let temp = format!( "{}/{}" , root , path.as_str() );
		// let res  = match copy_file_to_string(&temp) {
		// 	Ok(res)  => res,
        // 	Err(_)   => continue,
		// };


		let res = get_map_element(req_mapper, format!("/{}",path) );

		// println!("{}" , res);

		let par = node.parent().unwrap();

        par .append(create_css_node(&res));
        node.detach();
	}

}


/// Parses the objects contained in an HTML page.
pub fn parse_objects(document: &NodeRef, root: &str, uri: &str, alias: usize) -> Vec<Object> {

    // Objects vector
	let mut objects: Vec<Object> = Vec::with_capacity(10);
	let mut found_favicon        = false;

	// Find:
	// - <img> and <link href="favicon.ico" rel="shortcut icon">
	// - <link rel="stylesheet">
	// - <script src="...">

	for node_data in document.select("link").unwrap() {

		let node      = node_data.as_node();
		let path_attr = "href";

        let path = match node_get_attribute(node, path_attr) {
			Some(p) if p != "" && !p.starts_with("data:") => p       ,
			_                                             => continue,
		};

		if path.contains("favicon.ico") {
			continue;
		}

		let temp = format!( "{}/{}" , root , path.as_str() );
		let res  = match copy_file_to_string(&temp) {
			Ok(res)  => res,
        	Err(_)   => continue,
		};

		// println!("{}" , res);

		let par = node.parent().unwrap();

        par .append(create_css_node(&res));
        node.detach();
	}

    for node_data in document.select("img,link,script").unwrap() {

        let node = node_data.as_node();
		let name = node_data.name.local.to_lowercase();

		let path_attr = if name == "link" { "href" } else { "src" };
		let path      = match node_get_attribute(node, path_attr) {
			Some(p) if p != "" && !p.starts_with("data:") => p       ,
			_                                             => continue,
		};

		let rel  = node_get_attribute(node, "rel").unwrap_or_default();
		let kind = match ( name.as_str(), rel.as_str() ) {
			("link", "stylesheet")                       => ObjectKind::CSS                          ,
			("link", "shortcut icon") | ("link", "icon") => { found_favicon = true; ObjectKind::IMG },
			("script", _)                                => ObjectKind::JS                           ,
			("img", _)                                   => ObjectKind::IMG                          ,
			_                                            => continue                                 ,
		};

		/* Consider the posibility that the css file already has some GET parameters */
		let split: Vec<&str> = path.split('?').collect();
		let relative         = split[0];
		let fullpath;

		match uri_to_abs_fs_path(root, relative, uri, alias) {
			Some(absolute) => fullpath = absolute,
			None           => continue
		}

		match aux::stringify_error( fs::read(&fullpath) ) {
			Ok(data) => objects.push( Object::existing(&data, kind, path, node) ),
			Err(e)   => { eprint!("libalpaca: cannot read {} ({})\n", fullpath, e); continue },
		}
	}

    for node_data in document.select("style").unwrap() {

		let node         = node_data .as_node();
		let last_child   = node_data .as_node().last_child().unwrap();
		let refc         = last_child.into_text_ref().unwrap();

        let refc_val     = refc.borrow();
		let images_paths = css_parse_all_images(&refc_val);

		for path in images_paths {

			// println!("{}",path);

			let kind = ObjectKind::CSS;

			let split: Vec<&str> = path.split('?').collect();
			let relative         = split[0];
			let fullpath;

			match uri_to_abs_fs_path(root, relative, uri, alias) {
				Some(absolute) => fullpath = absolute,
				None           => continue
			}

			match aux::stringify_error(fs::read(&fullpath)) {
				Ok(data) => objects.push( Object::existing(&data, kind, path, node) ),
				Err(e)   => { eprint!("libalpaca: cannot read {} ({})\n", fullpath, e); continue },
			}
		}
	}

	// If no favicon was found, insert an empty one
	if !found_favicon {
		insert_empty_favicon(document);
	}

    objects.sort_unstable_by( |a, b| b.content.len().cmp( &a.content.len() ) ); // larger first
	objects
}


pub fn insert_empty_favicon(document: &NodeRef) {

    // Append the <link> either to the <head> tag, if exists, otherwise
    // to the whole document
    let node_data;  // to outlive the match
    let node = match document.select("head").unwrap().next() {
        Some(nd) => { node_data = nd; node_data.as_node() },
        None     => document                               ,
    };

	let elem = create_element("link");

    node_set_attribute( &elem, "href", String::from("data:,")       );
	node_set_attribute( &elem, "rel", String::from("shortcut icon") );

    node.append(elem);
}

/// Maps a (relative or absolute) uri, to an absolute filesystem path.
/// Returns None if uri_path is located in another server
fn uri_to_abs_fs_path(root: &str, relative: &str, page_uri: &str, alias: usize) -> Option<String> {

	if relative.starts_with("https://") || relative.starts_with("http://") {
		return None;
	}

	let mut fs_relative = String::from(relative);

	if !fs_relative.starts_with('/') {

        let base = Path::new(page_uri).parent().unwrap().to_str().unwrap();

		if !base.ends_with('/') {
			fs_relative.insert(0,'/');
		}

        fs_relative.insert_str(0,base);
	}

	// Resolve the dots in the path so far
	let     components: Vec<&str>   = fs_relative.split("/").collect(); 	// Original components of the path
	let mut normalized: Vec<String> = Vec::with_capacity(components.len()); // Stack to be used for the normalization

	for comp in components {
		if comp == "." || comp == "" {
            continue;
        } else if comp == ".." {
			if !normalized.is_empty() {
				normalized.pop();
			}
		} else {
			normalized.push("/".to_string()+comp);
		}
	}

	let mut absolute: String = normalized.into_iter().collect(); // String with the resolved relative path

	if page_uri[..alias] != absolute[..alias] {
		return None;
	}

	absolute = absolute[alias..].to_string(); // Remove alias characters in case there are any

	absolute.insert_str(0,root); // Make the above path absolute by adding the root

	Some(absolute)
}

pub fn parse_html(input: &str) -> NodeRef {

    let mut opts = ParseOpts::default();
    opts.tree_builder.drop_doctype = true;

    let mut parser = parse_html_with_options(opts);
    parser.process(input.into());
    parser.finish()
}

pub fn serialize_html(dom: &NodeRef) -> Vec<u8> {

    let mut buf: Vec<u8> = Vec::new();
    let opts             = SerializeOpts::default();

    serialize(&mut buf, dom, opts).expect("serialization failed");

    buf
}

pub fn create_element(name: &str) -> NodeRef {
    let qual_name = QualName::new( None, ns!(), LocalName::from(name) );
    NodeRef::new_element(qual_name, Vec::new())
}

pub fn node_get_attribute(node: &NodeRef, name: &str) -> Option<String> {

    match node.as_element() {
        Some(element) => {
            match element.attributes.borrow().get(name) {
                Some(val) => Some( String::from(val) ),
                None      => None,
            }
        },
        None => None,
    }
}

pub fn node_set_attribute(node: &NodeRef, name: &str, value: String) {
    let elem = node.as_element().unwrap();
    elem.attributes.borrow_mut().insert(name, value);
}
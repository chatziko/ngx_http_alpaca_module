//! Contains main morphing routines.
use std::ffi::CStr;
use pad::{get_html_padding, get_object_padding};
use dom;
use pad;
use dom::{Object,ObjectKind,node_get_attribute};
use distribution::{Dist, sample_ge, sample_pair_ge, sample_ge_many};
use deterministic::*;
use aux::stringify_error;
use base64;
use std::fs;

// use image::gif::{GifDecoder, GifEncoder};
// use image::{ImageDecoder, AnimationDecoder};
// use std::fs::File;


use kuchiki::NodeRef;

#[repr(C)]
pub struct MorphInfo {
    // request info
    content: *const u8,     // u8 = uchar
    size: usize,
    root: *const u8,
    uri: *const u8,
    http_host: *const u8,
    alias: usize,
    query: *const u8,       // part after ?
    content_type: *const u8,

    probabilistic: usize,   // boolean

    // for probabilistic
    dist_html_size: *const u8,
    dist_obj_num: *const u8,
    dist_obj_size: *const u8,
    use_total_obj_size: usize,

    // for deterministic
    obj_num: usize,
    obj_size: usize,
    max_obj_size: usize,

    //for object inlining
    inlining_obj_num: usize,
}

fn keep_local_objects(objects : &mut Vec<Object>){
    objects.retain(|obj| !obj.uri.contains("http:") && !obj.uri.contains("https:") )
}

fn get_file_extension(file_name : &String) -> String{
    let mut split: Vec<&str> = file_name.split(".").collect();
    split.pop().unwrap().to_owned()
}

fn get_img_format_and_ext(file_full_path : &String , file_name : &String) -> String {
    
    let base_img = fs::read(file_full_path).expect("Unable to read file");

    let extent = get_file_extension(&file_name);

    let ext : String;
    match extent.as_str() {
        "jpg" | "jpeg" => {
            ext = String::from("jpeg");
        },
        "png" => {
            ext = String::from("png");
        }
        "gif" => {
            ext = String::from("gif");
        }
        _ => panic!("unknown image type"),
    }; 

    let res_base64 = base64::encode(&base_img);

    let temp = format!("data:image/{};charset=utf-8;base64,{}",ext,res_base64);
    
    temp
}

/// It samples a new page using probabilistic morphing, changes the
/// references to its objects accordingly, and pads it.
#[no_mangle]
pub extern "C" fn morph_html(pinfo: *mut MorphInfo) -> u8 {

    
    std::env::set_var("RUST_BACKTRACE", "full");
    let info = unsafe { &mut *pinfo };

    let root = c_string_to_str(info.root).unwrap();
    let uri = c_string_to_str(info.uri).unwrap();
    let http_host = c_string_to_str(info.http_host).unwrap();

    // /* Convert arguments into &str */
    let html = match c_string_to_str(info.content) {
        Ok(s) => s,
        Err(e) => {
            eprint!("libalpaca: cannot read html content of {}: {}\n", uri, e);
            return 0;       // return NULL pointer if html cannot be converted to a string
        }
    };

    let document = dom::parse_html(html);

    let full_root = String::from(root).replace("$http_host", http_host);

    let mut objects = dom::parse_objects(&document, full_root.as_str(), uri, info.alias); // Vector of objects found in the html.

    keep_local_objects(&mut objects);
    let mut orig_n = objects.len(); // Number of original objects.

    println!("OBJ NUM {}" , orig_n);

    let target_size = match
        if info.probabilistic != 0 {
            morph_probabilistic(&document, &mut objects, &info)
        } else {
            morph_deterministic(&document, &mut objects, &info)
        } {
        Ok(s) => s,
        Err(e) => {
            eprint!("libalpaca: cannot morph: {}\n", e);
            return document_to_c(&document, info);
        }
    };

    // insert refs and add padding
    if info.inlining_obj_num > orig_n {
        info.inlining_obj_num = orig_n;
    }
    match make_objects_inlined(&mut objects, full_root.as_str() , info.inlining_obj_num) {
        Ok(_) => {
            orig_n = orig_n - info.inlining_obj_num;
        },
        Err(e) => {
            eprint!("libalpaca: insert_objects_refs failed: {}\n", e);
            return document_to_c(&document, info);
        }
    }

    println!("NEW OBJ NUM {}" , orig_n);


    // insert refs and add padding
    match insert_objects_refs(&document, &objects, orig_n) {
        Ok(_) => {},
        Err(e) => {
            eprint!("libalpaca: insert_objects_refs failed: {}\n", e);
            return document_to_c(&document, info);
        }
    }

    let mut content = dom::serialize_html(&document);
    get_html_padding(&mut content, target_size); // Pad the html to the target size.

    return content_to_c(content, info);
}

/// Inserts the ALPaCA GET parameters to the html objects, and adds the fake objects to the html.
fn make_objects_inlined(objects: &mut Vec<Object>, root: &str , n: usize) -> Result<(), String> {

    let obj_for_inlining = &objects[0..n]; // Slice which contains initial objects
    let mut objects_inlined = Vec::new();
    // let rest_obj = &objects[n..]; // Slice which contains ALPaCA objects

    for (i,object) in obj_for_inlining.iter().enumerate() {
        // ignore objects without target size
        if !object.target_size.is_none() {

            println!("{}",object.uri);

            let node = object.node.as_ref().unwrap();
            let attr = match node.as_element().unwrap().name.local.to_lowercase().as_ref() {
                "img" | "script" => "src",
                "link" => "href",
                "style" => "style",
                _ => panic!("shouldn't happen"),
            };

            let path : String;
            if attr != "style" {
                path = match node_get_attribute(node, attr) {
                    Some(p) if p != "" && !p.starts_with("data:") => p,
                    _ => continue,
                };
            }
            else {
                path = object.uri.clone();
            }

            let temp = format!("{}/{}" , root , path.as_str());

            println!("{}", temp);

            let temp = get_img_format_and_ext(&temp , &object.uri);
         
            if attr != "style" {

                dom::node_set_attribute(node, attr, temp);

                objects_inlined.push(i);
            }
            else {
        
                let last_child = node.last_child().unwrap();
                let refc = last_child.into_text_ref().unwrap();
                
                let mut refc_val = refc.borrow().clone();
        
                refc_val = refc_val.replace(&object.uri , &temp);

                // println!("{}", refc_val);
        
                *refc.borrow_mut() = refc_val;

                objects_inlined.push(i);
            }
        }
    }
    

    for _ in objects_inlined.clone() {
        objects.remove(objects_inlined.pop().unwrap());
    }

    Ok(())
}

/// Returns the object's padding.
#[no_mangle]
pub extern "C" fn morph_object(pinfo: *mut MorphInfo) -> u8 {

    let info = unsafe { &mut *pinfo };

    let content_type = c_string_to_str(info.content_type).unwrap();
    let query = c_string_to_str(info.query).unwrap();

    let kind = dom::parse_object_kind(content_type);

    let target_size = dom::parse_target_size(query);
    if (target_size == 0) || (target_size <= info.size) {
        // Target size has to be greater than current size.
        eprint!("alpaca: morph_object: target_size ({}) cannot match current size ({})\n", target_size, info.size);
        return content_to_c(Vec::new(), info);
    }

    let padding = get_object_padding(kind, info.size, target_size); // Get the padding for the object.

    return content_to_c(padding, info);
}

/// Frees memory allocated in rust.
#[no_mangle]
pub extern "C" fn free_memory(data: *mut u8, size: usize) {

    let s = unsafe { std::slice::from_raw_parts_mut(data, size) };
    let s = s.as_mut_ptr();
    unsafe {
        Box::from_raw(s);
    }
}

fn morph_probabilistic (
    document: &NodeRef,
    objects: &mut Vec<Object>,
    info: &MorphInfo,
) -> Result<usize, String> {

    let dist_html_size = Dist::from(c_string_to_str(info.dist_html_size)?)?;
    let dist_obj_num = Dist::from(c_string_to_str(info.dist_obj_num)?)?;
    let dist_obj_size = Dist::from(c_string_to_str(info.dist_obj_size)?)?;

    // we'll have at least as many objects as the original ones
    let initial_obj_num = objects.len();

    // Sample target number of objects (count)
    let mut target_obj_num = match sample_ge(&dist_obj_num, initial_obj_num) {
        Ok(c) => c,
        Err(e) => {
            eprint!("libalpaca: could not sample object number ({}), leaving unchanged ({})\n", e, initial_obj_num);
            initial_obj_num
        }
    };

    // sample target html size
    let content = dom::serialize_html(&document);
    let min_html_size = content.len()
        + 7                                         // for the comment characters
        + 23 * initial_obj_num                       // for ?alpaca-padding=...
        + 94 * (target_obj_num - initial_obj_num);   // for the fake images
    let target_html_size;

    // find object sizes
    if info.use_total_obj_size == 0 {
        // Sample each object size from dist_obj_size.
        //
        target_html_size = sample_ge(&dist_html_size, min_html_size)?;

        // To more closely match the actual obj_size distribution, we'll sample values for all objects,
        // And then we'll use the largest to pad existing objects and the smallest for padding objects.
        let mut target_obj_sizes: Vec<usize> = sample_ge_many(&dist_obj_size, 1, target_obj_num)?;
        target_obj_sizes.sort_unstable();       // ascending

        // Pad existing objects
        for obj in &mut *objects {
            let needed_size = obj.content.len() + pad::min_obj_padding(&obj);

            // Take the largest size, if not enough draw a new one with this specific needed_size
            obj.target_size = if target_obj_sizes[target_obj_sizes.len()-1] >= needed_size {
                Some(target_obj_sizes.pop().unwrap())
            } else {
                match sample_ge(&dist_obj_size, needed_size) {
                    Ok(size) => Some(size),
                    Err(e) => {
                        eprint!("libalpaca: warning: no padding was found for {} ({})\n", obj.uri, e);
                        None
                    },
                }
            };
        }

        // create padding objects, using the smallest of the sizes
        for i in 0..target_obj_num - initial_obj_num {
            objects.push(Object::fake_image(target_obj_sizes[i]));
        }

    } else {
        // Sample the __total__ object size from dist_obj_size.

        // min size of all objects
        let min_obj_size = objects.into_iter().map(
            |obj| obj.content.len() + pad::min_obj_padding(obj)
        ).sum();
        let target_obj_size;

        // sample html/obj sizes, either together or separately
        if dist_obj_size.name == "Joint" {
            match sample_pair_ge(&dist_html_size, (min_html_size, min_obj_size))? {
                (a, b) => {
                    target_html_size = a;
                    target_obj_size = b;
                }
            }
        } else {
            target_html_size = sample_ge(&dist_html_size, min_html_size)?;
            target_obj_size  = sample_ge(&dist_obj_size,  min_obj_size )?;
        }

        // create empty fake images
        if target_obj_size > 0 && target_obj_num == 0 {
            // we chose a non-zero target_obj_size but have no objects to pad, create a fake one
            target_obj_num = 1;
        }
        for _ in 0..target_obj_num - initial_obj_num {
            objects.push(Object::fake_image(0));
        }

        // split all extra size equally among all objects
        let mut to_split  = target_obj_size - min_obj_size;
        for (pos, obj) in objects.iter_mut().enumerate() {
            let pad = to_split / (target_obj_num - pos);
            obj.target_size = Some(obj.content.len() + pad::min_obj_padding(obj) + pad);
            to_split -= pad;
        }
    }

    Ok(target_html_size)
}

fn morph_deterministic(
    document: &NodeRef,
    objects: &mut Vec<Object>,
    info: &MorphInfo,
) -> Result<usize, String> {
    // we'll have at least as many objects as the original ones
    let initial_obj_no = objects.len();

    // Sample target number of objects (count) and target sizes for morphed
    // objects. Count is a multiple of "obj_num" and bigger than "min_count".
    // Target size for each objects is a multiple of "obj_size" and bigger
    // than the object's  original size.
    let target_count = get_multiple(info.obj_num, initial_obj_no);

    for i in 0..objects.len() {
        let min_size = objects[i].content.len()
            + match objects[i].kind { ObjectKind::CSS | ObjectKind::JS => 4, _ => 0 };

        let obj_target_size = get_multiple(info.obj_size, min_size);
        objects[i].target_size = Some(obj_target_size);
    }

    let fake_objects_count = target_count - initial_obj_no; // The number of fake objects.

    // To get the target size of each fake object, sample uniformly a multiple
    // of "obj_size" which is smaller than "max_obj_size".
    let fake_objects_sizes = get_multiples_in_range(info.obj_size, info.max_obj_size, fake_objects_count)?;

    // Add the fake objects to the vector.
    for i in 0..fake_objects_count {
        objects.push(Object::fake_image(fake_objects_sizes[i]));
    }

    // find target size,a multiple of "obj_size".
    let content = dom::serialize_html(&document);
    let html_min_size = content.len() + 7; // Plus 7 because of the comment characters.
    Ok(get_multiple(info.obj_size, html_min_size))
}

/// Inserts the ALPaCA GET parameters to the html objects, and adds the fake objects to the html.
fn insert_objects_refs(document: &NodeRef, objects: &[Object], n: usize) -> Result<(), String> {
    let init_obj = &objects[0..n]; // Slice which contains initial objects
    let padding_obj = &objects[n..]; // Slice which contains ALPaCA objects

    for object in init_obj {
        // ignore objects without target size
        if !object.target_size.is_none() {
            append_ref(&object);
        }
    }

    add_padding_objects(&document, padding_obj);

    Ok(())
}


/// Appends the ALPaCA GET parameter to an html element
fn append_ref(object: &Object) {
    // Construct the link with the appended new parameter
    let mut new_link = String::from("alpaca-padding=");
    new_link.push_str(&(object.target_size.unwrap().to_string())); // Append the target size

    let node = object.node.as_ref().unwrap();
    let attr = match node.as_element().unwrap().name.local.to_lowercase().as_ref() {
        "img" | "script" => "src",
        "link" => "href",
        "style" => "style",
        _ => panic!("shouldn't happen"),
    };

    // Check if there is already a GET parameter in the file path
    let prefix = if object.uri.contains("?") { '&' } else { '?' };

    new_link.insert(0, prefix);
    new_link.insert_str(0, &object.uri);

    if attr != "style" {
        dom::node_set_attribute(node, attr, new_link);
    }
    else {

        let last_child = node.last_child().unwrap();
        let refc = last_child.into_text_ref().unwrap();
        
        let mut refc_val = refc.borrow().clone();

        refc_val = refc_val.replace(&object.uri , &new_link);

        *refc.borrow_mut() = refc_val;

        // println!("{}", refc.borrow());
    }

}

//// Adds the fake ALPaCA objects in the end of the html body
fn add_padding_objects(document: &NodeRef, objects: &[Object]) {

    // append the objects either to the <body> tag, if exists, otherwise
    // to the whole document
    let node_data;  // to outlive the match
    let node = match document.select("body").unwrap().next() {
        Some(nd) => { node_data = nd; node_data.as_node() },
        None => document,
    };

    let mut i = 1;
    for object in objects {
        let elem = dom::create_element("img");
        dom::node_set_attribute(&elem, "src", format!("/__alpaca_fake_image.png?alpaca-padding={}&i={}", object.target_size.unwrap(), i));
        dom::node_set_attribute(&elem, "style", String::from("visibility:hidden"));
        node.append(elem);
        i += 1;
    }
}

// Builds the returned html, stores its size in html_size and returns a
// 'forgotten' unsafe pointer to the html, for returning to C
//
fn document_to_c(document: &NodeRef, info: &mut MorphInfo) -> u8 {
    let content = dom::serialize_html(document);
    return content_to_c(content, info);
}

fn content_to_c(content: Vec<u8>, info: &mut MorphInfo) -> u8 {
    info.size = content.len();

    let mut buf = content.into_boxed_slice();
    info.content = buf.as_mut_ptr();
    std::mem::forget(buf);
    1
}

fn c_string_to_str<'a>(s: *const u8) -> Result<&'a str, String> {
    return stringify_error(unsafe { CStr::from_ptr(s as *const i8) }.to_str());
}
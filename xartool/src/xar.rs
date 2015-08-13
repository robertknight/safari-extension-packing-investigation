extern crate byteorder;
extern crate flate2;
extern crate sxd_document;

use std::io::{Cursor, Read, Seek, SeekFrom};
use std::error::Error;
use std::str::FromStr;

use self::byteorder::{BigEndian, ReadBytesExt};
use self::flate2::read::ZlibDecoder;
use self::sxd_document::dom::{ChildOfRoot, ChildOfElement, Element};

#[derive(Debug)]
pub struct Header {
    magic: u32,
    size: u16,
    version: u16,
    toc_length_compressed: u64,
    toc_length_uncompressed: u64,
    checksum_algorithm: u32,
}

/// A reference to a location within the data
/// section of the archive (the heap).
#[derive(Debug)]
pub struct HeapRef {
    pub size: u64,
    pub offset: u64
}

/// Metadata about a block of compressed data
/// within the archive's heap.
#[derive(Debug)]
pub struct HeapData {
    pub location: HeapRef,
    pub length: u64,
    pub archived_checksum: String,
    pub extracted_checksum: String,
    pub encoding: Encoding
}

#[derive(Debug)]
pub enum Encoding {
    Gzip,
    Other
}

#[derive(Debug)]
pub struct Checksum {
    pub location: HeapRef,
    pub style: String,
    pub data: Vec<u8>
}

#[derive(Debug)]
pub struct Signature {
    pub location: HeapRef,
    pub style: String,
    pub x509_certs: Vec<String>,
    pub data: Vec<u8>
}

#[derive(Debug)]
pub enum FileType {
    File,
    Directory,
    Other
}

#[derive(Debug)]
pub struct File {
    // FIXME - A file may be a normal file or directory,
    // this struct mirrors the XML but is not idiomatic Rust
    pub name: String,
    pub file_type: FileType,
    pub children: Vec<File>,
    pub data: Option<HeapData>,
}

#[derive(Debug)]
pub struct Archive {
    pub source: Box<Read+Seek>,
    pub header: Header,

    pub checksum: Option<Checksum>,
    pub signature: Option<Signature>,
    pub files: Vec<File>
}

const XAR_MAGIC: u32 = 0x78617221;

//const XAR_CHECKSUM_NONE: u32 = 0;
const XAR_CHECKSUM_SHA1: u32 = 1;
//const XAR_CHECKSUM_MD5: u32 = 2;
//const XAR_CHECKSUM_OTHER: u32 = 3;

fn read_toc<R: Read>(source: &mut R, len: u64) -> String {
    let mut toc_data = vec![0u8; len as usize];
    source.read(&mut toc_data).unwrap();
    let toc_cursor = Cursor::new(toc_data);
    let mut decoder = ZlibDecoder::new(toc_cursor);
    let mut xml = String::new();
    decoder.read_to_string(&mut xml);
    xml
}

struct ChildElementIterator<'d> {
    children: Vec<ChildOfElement<'d>>,
    idx: usize
}

fn child_elements<'d>(elt: Element<'d>) -> ChildElementIterator<'d> {
    ChildElementIterator {
        children: elt.children(),
        idx: 0
    }
}

impl <'d> Iterator for ChildElementIterator<'d> {
    type Item = Element<'d>;

    fn next(&mut self) -> Option<Element<'d>> {
        while self.idx < self.children.len() {
            let node = self.children[self.idx];
            self.idx += 1;
            if let Some(elt) = node.element() {
                return Some(elt);
            }
        }
        None
    }
}

fn find_child<'d>(elt: Element<'d>, name: &str) -> Option<Element<'d>> {
    for child in elt.children() {
        match child {
            sxd_document::dom::ChildOfElement::Element(e) => {
                if e.name().local_part() == name {
                    return Some(e);
                }
            },
            _ => ()
        }
    }
    None
}

fn read_elt_text<'d>(elt: Element<'d>) -> String {
    for child in elt.children() {
        if let Some(text) = child.text() {
            return text.text().to_string();
        }
    }
    String::new()
}

fn read_child_elt_str<'d>(elt: Element<'d>, name: &str) -> String {
    if let Some(child_elt) = find_child(elt, name) {
        return read_elt_text(child_elt);
    };
    String::new()
}

fn read_child_elt_value<'d, T>(elt: Element<'d>, name: &str) -> T 
where T: FromStr + Default {
    match read_child_elt_str(elt, name).parse::<T>() {
        Ok(val) => val,
        Err(_) => Default::default()
    }
}

fn parse_heap_ref<'d>(elt: Element<'d>) -> HeapRef {
    let offset: u64 = read_child_elt_value(elt, "offset");
    let size: u64 = read_child_elt_value(elt, "size");

    HeapRef {
        offset: offset,
        size: size
    }
}

fn parse_checksum<'d>(elt: Element<'d>) -> Checksum {
    Checksum {
        location: parse_heap_ref(elt),
        style: elt.attribute_value("style").unwrap_or_default().to_string(),
        data: Vec::new()
    }
}

fn parse_signature<'d>(elt: Element<'d>) -> Signature {
    let mut certs = Vec::new();

    if let Some(key_info_elt) = find_child(elt, "KeyInfo") {
        if let Some(x509_data_elt) = find_child(key_info_elt, "X509Data") {
            for cert_elt in child_elements(x509_data_elt) {
                certs.push(read_elt_text(cert_elt));
            }
        }
    }

    Signature {
        location: parse_heap_ref(elt),
        style: elt.attribute_value("style").unwrap_or_default().to_string(),
        x509_certs: certs,
        data: Vec::new()
    }
}

fn parse_heap_data<'d>(elt: Element<'d>) -> HeapData {
    let encoding = if let Some(encoding_elt) = find_child(elt, "encoding") {
        match encoding_elt.attribute_value("style").unwrap_or_default() {
            "application/x-gzip" => Encoding::Gzip,
            _ => Encoding::Other
        }
    } else {
        Encoding::Other
    };

    HeapData {
        location: parse_heap_ref(elt),
        length: read_child_elt_value(elt, "length"),
        archived_checksum: read_child_elt_str(elt, "archived-checksum"),
        extracted_checksum: read_child_elt_str(elt, "extracted-checksum"),
        encoding: encoding
    }
}

fn parse_file<'d>(elt: Element<'d>) -> File {
    let mut file = File {
        name: String::new(),
        file_type: FileType::Other,
        data: None,
        children: Vec::new()
    };

    for child_elt in child_elements(elt) {
        match child_elt.name().local_part() {
            "file" => file.children.push(parse_file(child_elt)),
            "name" => file.name = read_elt_text(child_elt),
            "type" => {
                let text: &str = &read_elt_text(child_elt);
                file.file_type = match text {
                    "file" => FileType::File,
                    "directory" => FileType::Directory,
                    _ => FileType::Other
                }
            },
            "data" => file.data = Some(parse_heap_data(child_elt)),
            _ => ()
        }
    }

    file
}

impl Archive {
    pub fn open<Source>(mut source: Source) -> Result<Archive, Box<Error>>
      where Source: Read + Seek {
          // read header
          let header = Header {
            magic: try!(source.read_u32::<BigEndian>()),
            size: try!(source.read_u16::<BigEndian>()),
            version: try!(source.read_u16::<BigEndian>()),
            toc_length_compressed: try!(source.read_u64::<BigEndian>()),
            toc_length_uncompressed: try!(source.read_u64::<BigEndian>()),
            checksum_algorithm: try!(source.read_u32::<BigEndian>())
          };

          if header.magic != XAR_MAGIC {
              println!("not a XAR archive");
          }

          if header.version != 1 {
              println!("unsupported archive version {}", header.version)
          }
          
          if header.checksum_algorithm != XAR_CHECKSUM_SHA1 {
              println!("unsupported checksum type {}", header.checksum_algorithm)
          }

          // read table of contents, checksum and signature
          source.seek(SeekFrom::Start(header.size as u64));
          let toc_xml = read_toc(&mut source,
                                 header.toc_length_compressed).replace("encoding=\"UTF-8\"","");
          let xml_parser = sxd_document::parser::Parser::new();
          let toc_package = match xml_parser.parse(&toc_xml) {
              Ok(package) => package,
              Err((offset, errs)) => {
                  println!("failed to parse TOC. error at location {}: {:?}", offset, errs);
                  sxd_document::Package::new()
              }
          };

          let mut archive_checksum: Option<Checksum> = None;
          let mut archive_signature: Option<Signature> = None;
          let mut files = Vec::new();

          let toc_document = toc_package.as_document();
          let xar_root = toc_document.root().children().iter().find(|elt| match **elt {
            ChildOfRoot::Element(e) => e.name().local_part() == "xar",
            _ => false
          }).unwrap().element();
          let toc_root = find_child(xar_root.unwrap(), "toc").unwrap();

          for toc_element in toc_root.children() {
              if let ChildOfElement::Element(child_elt) = toc_element {
                  match child_elt.name().local_part() {
                      "checksum" => {
                          let mut checksum = parse_checksum(child_elt);
                          checksum.data = vec![0u8; checksum.location.size as usize];
                          source.seek(SeekFrom::Start((header.size as u64) +
                                                      checksum.location.offset));
                          source.read(&mut checksum.data);
                          archive_checksum = Some(checksum);
                      },
                      "signature" => {
                          let mut sig = parse_signature(child_elt);
                          sig.data = vec![0u8; sig.location.size as usize];
                          source.seek(SeekFrom::Start((header.size as u64) + sig.location.offset));
                          source.read(&mut sig.data);
                          archive_signature = Some(sig)
                      },
                      "file" => {
                          files.push(parse_file(child_elt));
                      }
                      _ => ()
                  }
              }
          }

          let archive = Archive {
              header: header,
              checksum: archive_checksum,
              signature: archive_signature,
              files: files
          };
          Ok(archive)
    }
}


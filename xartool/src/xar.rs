extern crate byteorder;
extern crate flate2;
extern crate sha1;
extern crate sxd_document;

use std::io::{Cursor, Read, Seek, SeekFrom};
use std::error::Error;
use std::str::{from_utf8, FromStr};

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
#[derive(Debug, Clone)]
pub struct HeapRef {
    pub offset: u64,
    /// The size of the data _after_ decompression (if any)
    pub size: u64
}

/// Metadata about a block of compressed data
/// within the archive's heap.
#[derive(Debug, Clone)]
pub struct HeapData {
    pub location: HeapRef,
    /// The length of the file's compressed content
    pub length: u64,
    pub archived_checksum: String,
    pub extracted_checksum: String,
    pub encoding: Encoding
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum FileType {
    File,
    Directory,
    Other
}

#[derive(Debug, Clone)]
pub struct File {
    // FIXME - A file may be a normal file or directory,
    // this struct mirrors the XML but is not idiomatic Rust
    pub name: String,
    pub file_type: FileType,
    pub children: Vec<File>,
    pub data: Option<HeapData>,
}

pub struct Archive<R: Read+Seek> {
    source: R,

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

fn to_hex(input: &[u8]) -> String {
    let mut s = String::new();
    for b in input.iter() {
        s.push_str(&format!("{:02x}", *b));
    }
    return s;
}

fn sha1_digest(input: &[u8]) -> String {
    let mut digest = sha1::Sha1::new();
    digest.update(input);
    digest.hexdigest()
}

fn read_heap<R: Read + Seek>(source: &mut R, offset: u64, len: u64) -> Vec<u8> {
    let mut data = vec![0u8; len as usize];
    source.seek(SeekFrom::Start(offset));
    source.read(&mut data).unwrap();
    data
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

fn decompress_zlib(data: &[u8]) -> Vec<u8> {
    let cursor = Cursor::new(data);
    let mut decoder = ZlibDecoder::new(cursor);
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf);
    buf
}

impl <R:Read+Seek> Archive<R> {
    pub fn open(mut source: R) -> Result<Archive<R>, Box<Error>> {
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
          let toc_data = read_heap(&mut source, header.size as u64, header.toc_length_compressed);
          let toc_cursor = Cursor::new(toc_data);
          let mut decoder = ZlibDecoder::new(toc_cursor);
          let mut toc_xml = String::new();
          decoder.read_to_string(&mut toc_xml);
          toc_xml = toc_xml.replace("encoding=\"UTF-8\"","");

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

          let heap_offset = (header.size as u64) + header.toc_length_compressed;

          for toc_element in toc_root.children() {
              if let ChildOfElement::Element(child_elt) = toc_element {
                  match child_elt.name().local_part() {
                      "checksum" => {
                          let mut checksum = parse_checksum(child_elt);
                          checksum.data = read_heap(&mut source, heap_offset + checksum.location.offset,
                                                    checksum.location.size);
                          archive_checksum = Some(checksum);
                      },
                      "signature" => {
                          let mut sig = parse_signature(child_elt);
                          sig.data = read_heap(&mut source, heap_offset + sig.location.offset,
                                               sig.location.size);
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
              source: source,
              header: header,
              checksum: archive_checksum,
              signature: archive_signature,
              files: files
          };
          Ok(archive)
    }

    fn heap_offset(&self) -> u64 {
        self.header.size as u64 + self.header.toc_length_compressed
    }

    fn verify_file_tree(&mut self, file: &File) -> Result<(),Vec<String>> {
        let mut errs: Vec<String> = Vec::new();
        match file.file_type {
            FileType::Directory => {
                for dir_file in &file.children {
                    match self.verify_file_tree(dir_file) {
                        Ok(_) => (),
                        Err(sub_tree_errs) => errs.extend(sub_tree_errs.iter().cloned())
                    }
                }
            },
            FileType::File => {
                if let Some(ref file_data_ref) = file.data {
                    let heap_offset = self.heap_offset();
                    let file_data = read_heap(&mut self.source, heap_offset +
                                              file_data_ref.location.offset,
                                              file_data_ref.length);
                    let archived_digest = sha1_digest(&file_data);

                    if archived_digest != file_data_ref.archived_checksum {
                        errs.push(format!("Digest mismatch for {}. Expected {}, actual {}",
                                          file.name, file_data_ref.archived_checksum, archived_digest));
                    }

                    let decompressed_file_data = decompress_zlib(&file_data);
                    let extracted_digest = sha1_digest(&decompressed_file_data);

                    if extracted_digest != file_data_ref.extracted_checksum {
                        errs.push(format!("Extracted digest mismatch for {}. Expected {}, actual {}",
                                          file.name, file_data_ref.extracted_checksum, extracted_digest));
                    }
                }
            },
            FileType::Other => ()
        }
        match errs.len() {
            0 => Ok(()),
            _ => Err(errs)
        }
    }

    pub fn verify(&mut self) -> Result<(),String> {
        // verify TOC checksum
        if let Some(ref checksum) = self.checksum {
            let toc_data = read_heap(&mut self.source, self.header.size as u64,
                                     self.header.toc_length_compressed);
            let toc_digest = sha1_digest(&toc_data);
            let expected_checksum = to_hex(&checksum.data);

            if toc_digest != expected_checksum {
                return Err(format!("Checksum mismatch. Expected {}, actual {}", expected_checksum,
                                   toc_digest));
            }
        }

        // verify file data
        let files = self.files.clone();
        let mut verify_errs: Vec<String> = Vec::new();
        for file in files.iter() {
            match self.verify_file_tree(file) {
                Ok(_) => (),
                Err(errs) => verify_errs.extend(errs.iter().cloned())
            }
        }
        if verify_errs.len() > 0 {
            return Err(verify_errs.connect(", "));
        }

        // verify signature
        if let Some(_) = self.signature {
            println!("TODO - Verify signature")
        }

        Ok(())
    }
}


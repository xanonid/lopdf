use super::content::*;
use super::Object::*;
use super::{Dictionary, Document, ObjectId, Stream};
use image::{self, ColorType, GenericImage, ImageFormat};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub fn form(boundingbox: Vec<f64>, matrix: Vec<f64>, content: Vec<u8>) -> Stream {
	let mut dict = Dictionary::new();
	dict.set("Type", Name(b"XObject".to_vec()));
	dict.set("Subtype", Name(b"Form".to_vec()));
	dict.set("BBox", Array(boundingbox.into_iter().map(Real).collect()));
	dict.set("Matrix", Array(matrix.into_iter().map(Real).collect()));
	let mut xobject = Stream::new(dict, content);
	xobject.compress();
	xobject
}

pub fn image<P: AsRef<Path>>(path: P) -> Stream {
	let img = image::open(&path).unwrap();
	let (width, height) = img.dimensions();
	let (color_space, bits) = match img.color() {
		ColorType::Gray(bits) => (b"DeviceGray".to_vec(), bits),
		ColorType::RGB(bits) => (b"DeviceRGB".to_vec(), bits),
		ColorType::Palette(bits) => (b"Indexed".to_vec(), bits),
		ColorType::GrayA(bits) => (b"DeviceN".to_vec(), bits),
		ColorType::RGBA(bits) => (b"DeviceN".to_vec(), bits),
	};

	let mut dict = Dictionary::new();
	dict.set("Type", Name(b"XObject".to_vec()));
	dict.set("Subtype", Name(b"Image".to_vec()));
	dict.set("Width", width);
	dict.set("Height", height);
	dict.set("ColorSpace", Name(color_space));
	dict.set("BitsPerComponent", bits);

	let mut file = File::open(&path).unwrap();
	let mut buffer = Vec::new();
	file.read_to_end(&mut buffer).unwrap();

	let is_jpeg = match image::guess_format(&buffer) {
		Ok(format) => match format {
			ImageFormat::JPEG => true,
			_ => false,
		},
		Err(_) => false,
	};

	if is_jpeg {
		dict.set("Filter", Name(b"DCTDecode".to_vec()));
		Stream::new(dict, buffer)
	} else {
		let mut img_object = Stream::new(dict, img.raw_pixels());
		img_object.compress();
		img_object
	}
}

impl Document {
	pub fn insert_image(&mut self, page_id: ObjectId, img_object: Stream, position: (f64, f64), size: (f64, f64)) {
		let img_id = self.add_object(img_object);
		let img_name = format!("X{}", img_id.0);

		let mut content = self.get_and_decode_page_content(page_id);
		// content.operations.insert(0, Operation::new("q", vec![]));
		// content.operations.push(Operation::new("Q", vec![]));
		content.operations.push(Operation::new("q", vec![]));
		content
			.operations
			.push(Operation::new("cm", vec![size.0.into(), 0.into(), 0.into(), size.1.into(), position.0.into(), position.1.into()]));
		content.operations.push(Operation::new("Do", vec![Name(img_name.as_bytes().to_vec())]));
		content.operations.push(Operation::new("Q", vec![]));
		let modified_contnet = content.encode().unwrap();
		self.add_xobject(page_id, img_name, img_id);
		self.change_page_content(page_id, modified_contnet);
	}

	pub fn insert_form_object(&mut self, page_id: ObjectId, form_obj: Stream) {
		let form_id = self.add_object(form_obj);
		let form_name = format!("X{}", form_id.0);

		let mut content = self.get_and_decode_page_content(page_id);
		content.operations.insert(0, Operation::new("q", vec![]));
		content.operations.push(Operation::new("Q", vec![]));
		// content.operations.push(Operation::new("q", vec![]));
		content.operations.push(Operation::new("Do", vec![Name(form_name.as_bytes().to_vec())]));
		// content.operations.push(Operation::new("Q", vec![]));
		let modified_contnet = content.encode().unwrap();
		self.change_page_content(page_id, modified_contnet);
		self.add_xobject(page_id, form_name, form_id);
	}
}
#[test]
fn insert_image() {
	use super::xobject;
	let mut doc = Document::load("assets/example.pdf").unwrap();
	let pages = doc.get_pages();
	let page_id = *pages.get(&1).expect(&format!("Page {} not exist.", 1));
	let img = xobject::image("assets/pdf_icon.jpg");
	doc.insert_image(page_id, img, (100.0, 210.0), (400.0, 225.0));
	doc.save("test_5_image.pdf").unwrap();
}

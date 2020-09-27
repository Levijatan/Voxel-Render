use image::GenericImageView;

use std::os::raw::c_void;

use flamer::flame;

#[flame]
pub fn read_texture(path: String) -> (Vec<u8>, u32, u32) {
    let img = image::open(path).unwrap();
    let (width, height) = img.dimensions();
    (img.to_bytes(), width, height)
}

#[flame]
pub fn generate_texture(path: String) -> u32 {
    let (img, width, height) = read_texture(path);
    let mut texture: u32 = 0;
    unsafe {
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGB as i32,
            width as i32,
            height as i32,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            &img[0] as *const u8 as *const c_void,
        );
        gl::GenerateMipmap(gl::TEXTURE_2D);
    }

    texture
}

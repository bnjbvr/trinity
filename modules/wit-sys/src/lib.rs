mod wit {
    wit_bindgen::generate!("sys" in "../../wit/sys.wit");
    pub use self::sys::*;
}

pub use wit::rand_u64;

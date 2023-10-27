mod wit {
    wit_bindgen::generate!("sys-world" in "../../wit/sys.wit");
    pub use self::trinity::api::sys::*;
}

pub use wit::rand_u64;

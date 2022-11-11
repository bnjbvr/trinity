mod wit {
    wit_bindgen_guest_rust::generate!({
        import: "../../wit/imports.wit",
        name: "sys"
    });
    pub use self::imports::*;
}

pub use wit::rand_u64;

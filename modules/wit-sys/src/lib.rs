mod wit {
    wit_bindgen_guest_rust::generate!({
        import: "../../wit/sys.wit",
        name: "sys"
    });
    pub use self::sys::*;
}

pub use wit::rand_u64;
pub use wit::resolve_room;

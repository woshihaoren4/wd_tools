pub fn v4_raw() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

pub fn v4() -> String {
    v4_raw().to_string()
}

pub enum UuidV5Namespace {
    DNS,
    OID,
    URL,
    X500,
}

pub fn v5_raw(ns: UuidV5Namespace, name: &[u8]) -> uuid::Uuid {
    match ns {
        UuidV5Namespace::DNS => uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, name),
        UuidV5Namespace::OID => uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, name),
        UuidV5Namespace::URL => uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, name),
        UuidV5Namespace::X500 => uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, name),
    }
}
pub fn v5(ns: UuidV5Namespace, name: &[u8]) -> String {
    v5_raw(ns, name).to_string()
}

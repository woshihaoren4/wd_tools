pub trait ToStaticStr{
    fn to_static_str(self)->&'static str;
}

impl ToStaticStr for String{
    fn to_static_str(self) -> &'static str {
        let bs = self.into_boxed_str();
        Box::leak(bs)
    }
}
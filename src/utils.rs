
pub fn type_of<T>(_: &T) -> &str {
    std::any::type_name::<T>()
}

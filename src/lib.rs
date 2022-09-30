pub mod devii;
mod test_struct;


#[macro_use]
extern crate derive_builder;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

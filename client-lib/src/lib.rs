pub struct Hello<'a> {
    name: &'a str
}

impl Hello<'_>{
    pub fn say_hi(&self) -> String {
        format!("Hello {}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use crate::Hello;

    #[test]
    fn hello_says_hello() {
        let hello = Hello { name: "pal" };
        assert_eq!(hello.say_hi(), "Hello pal");
    }
}
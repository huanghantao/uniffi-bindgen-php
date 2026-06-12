use std::sync::Mutex;

pub fn greet(name: String) -> String {
    format!("Hello, {name}")
}

#[derive(Debug, Default)]
pub struct TextDocument {
    content: Mutex<String>,
}

impl TextDocument {
    pub fn new(initial_content: String) -> Self {
        Self {
            content: Mutex::new(initial_content),
        }
    }

    pub fn append(&self, value: String) {
        self.content
            .lock()
            .expect("content lock poisoned")
            .push_str(&value);
    }

    pub fn content(&self) -> String {
        self.content
            .lock()
            .expect("content lock poisoned")
            .clone()
    }

    pub fn len(&self) -> u32 {
        self.content()
            .chars()
            .count()
            .try_into()
            .expect("content length does not fit in u32")
    }

    pub fn is_empty(&self) -> bool {
        self.content
            .lock()
            .expect("content lock poisoned")
            .is_empty()
    }

    pub fn clear(&self) {
        self.content
            .lock()
            .expect("content lock poisoned")
            .clear();
    }
}

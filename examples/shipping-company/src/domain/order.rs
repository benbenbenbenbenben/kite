pub struct Order {
    pub id: u32,
    pub description: String,
}

impl Order {
    pub fn new(id: u32, description: String) -> Self {
        Order { id, description }
    }
    pub fn ship(&self) {
        println!("Shipping order {}: {}", self.id, self.description);
    }
}
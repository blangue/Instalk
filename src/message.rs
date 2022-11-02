pub trait Message{
    fn send(&mut self) -> Result<(), String>;
}
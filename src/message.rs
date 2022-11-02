use std::{net::TcpStream, sync::mpsc::Sender, io::Write};

use crate::action::Action;

pub trait Message{
    fn send(&mut self) -> Result<(), String>;
}

/* Message types */
pub struct MessageServer{
    canal_comm: Sender<Action>,
    action: Action
}
impl MessageServer {
    pub fn new(canal_comm: Sender<Action>, action: Action) -> MessageServer{
        MessageServer { canal_comm, action }
    }
}
impl Message for MessageServer {
    fn send(&mut self) -> Result<(), String> {
        self.canal_comm.send(self.action.clone()).map_err(|e|
            format!("Erreur de communication avec le serveur : {}", e)
        )?;
        Ok(())
    }
}

pub struct MessageClient{
    nom: String,
    flux: Sender<String>,
    msg: String
}
impl MessageClient {
    pub fn new(nom: String, flux: Sender<String>, msg: String) -> MessageClient{
        MessageClient { nom, flux, msg }
    }
}
impl Message for MessageClient {
    fn send(&mut self) -> Result<(), String> {
        self.flux.send(self.msg.clone()).map_err(|e|
            format!("Erreur lors de l'envoi du message à {} :'{}'\nErreur : {}\n", self.nom, self.msg, e)
        )?;
        debug!("Message envoyé à {} :'{}'\n", self.nom, self.msg);
        Ok(())
    }
}

pub struct MessageClientTcp{
    nom: String,
    flux: TcpStream,
    msg: String
}
impl MessageClientTcp {
    pub fn new(nom: String, flux: TcpStream, msg: String) -> MessageClientTcp{
        MessageClientTcp { nom, flux, msg }
    }
    pub fn set_message(&mut self, new_msg : String) {
        self.msg = new_msg;
    }
}
impl Message for MessageClientTcp {
    fn send(&mut self) -> Result<(), String> {
        self.flux.write(self.msg.clone().as_bytes()).map_err(|e|
            format!("Erreur lors de l'envoi du message à {} :'{}'\nErreur : {}\n", self.nom, self.msg, e)
        )?;
        debug!("Message envoyé à {} :'{}'\n", self.nom, self.msg);
        Ok(())
    }
}
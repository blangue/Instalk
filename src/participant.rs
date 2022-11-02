use std::sync::mpsc::Sender;
/* Struct Participant */
//Définition du Participant
pub struct Participant {
    pub nom: String,
    pub messagerie : Sender<String>,
}
impl Clone for Participant {
    fn clone(&self) -> Self {
        Self { nom: self.nom.clone(), messagerie: self.messagerie.clone() }
    }
}
//Constructeur du Participant
impl Participant {
    pub fn new(nom: String, messagerie: Sender<String>) -> Participant {
        Participant {
            nom,
            messagerie,
        }
    }
}
impl std::fmt::Debug for Participant {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.nom)
    }
}
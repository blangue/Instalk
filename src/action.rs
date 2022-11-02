use crate::participant::Participant;

/* Actions possibles du master */
#[derive(Clone)]
pub enum Action {
    Message(String, Participant),
    AjoutParticipant(Participant),
    SuppParticipant(Participant),
}
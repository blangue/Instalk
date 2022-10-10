/* Nécessaire au logger personnalisé */
#[macro_use]
extern crate clap;
extern crate colored;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;

use colored::Colorize;
use log::{Level, LevelFilter, Metadata, Record};

/* Nécessaire au serveur */
use std::io::{Read,Write, BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::env;
use std::thread;

struct OurLogger;
impl log::Log for OurLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }
    fn log(&self, rec: &Record) {
        if self.enabled(rec.metadata()) {
            match rec.level() {
                Level::Error => eprintln!("{} {}", "error:".red().bold(), rec.args()),
                Level::Warn => eprintln!("{} {}", "warn:".yellow().bold(), rec.args()),
                Level::Info => eprintln!("{} {}", "info:".yellow().bold(), rec.args()),
                Level::Debug => eprintln!("{} {}", "debug:".bright_black().bold(), rec.args()),
                Level::Trace => eprintln!("{} {}", "trace:".blue().bold(), rec.args()),
            }
        }
    }
    fn flush(&self) {}
}

/* Code by Bastien LANGUE */

/* Objet Participant */
//Définition du Participant
pub struct Participant {
    nom: String,
    messagerie : mpsc::Sender<String>,
}
//Constructeur du Participant
impl Participant {
    pub fn new(nom: String, messagerie: mpsc::Sender<String>) -> Participant {
        Participant {
            nom: nom,
            messagerie: messagerie,
        }
    }
}

/* Action possibles du master */
enum Action {
    Message(String, Participant),
    AjoutParticipant(Participant),
    SuppParticipant(Participant),
}

/* -------------------- MAIN -------------------- */
fn main() -> std::io::Result<()>{
    /* Configuration du logger amélioré */
    log::set_logger(&OurLogger).unwrap();
    log::set_max_level(LevelFilter::Trace);
    
    /* Récupération de l'argument PORT */
    let arg: Vec<String> = env::args().collect();
    let port = &arg[1].parse::<u16>().unwrap();
    info!("Démarrage du serveur sur le port {}.", port);
    
    /* Démarrage du serveur et de l'écoute du port */
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap_or_else(|err| {
        error!("Impossible de démarrer le serveur sur le port {}.", port);
        panic!("Erreur : {}", err);
    });
    info!("Serveur démarré sur le port {}, les interlocuteurs peuvent se connecter.", port);
    
    /* ------------- THREAD DU MASTER ------------- */
    let (vers_serveur, au_serveur) =  mpsc::channel();
    /* Canal de communication au serveur */
    let _master = thread::spawn(move||{
        //Clients connectés
        let mut clients : Vec<Participant> = Vec::new();
        //Parsing de ce qui pourra être transmis
        while let Some(action) = au_serveur.recv().ok() {
            match action {
                //Cas d'un message
                Action::Message(message, auteur) => {
                    //Relai du message à tous les autres participants
                    for client in &clients {
                        if client.nom != auteur.nom {
                            client.messagerie.send(format!("{}: {}\n", auteur.nom, message.clone()));
                        }
                    }
                },
                //Cas d'un nouveau client
                Action::AjoutParticipant(participant) => {
                    clients.push(participant);
                },
                //Cas d'un client qui se déconnecte
                Action::SuppParticipant(participant) => {
                    clients.retain(|client| client.nom != participant.nom);
                },
            }
        }
    });
    
    /* Gestion des nouvelles connexions */
    for (id, flux) in listener.incoming().enumerate() {
        //Création du canal de communication avec le nouveau client
        let vers_serveur = vers_serveur.clone();
        let (vers_clients, au_client) = mpsc::channel();
        
        vers_serveur.send(Action::AjoutParticipant(
            Participant::new(format!("Client {}", id), vers_clients.clone()
        ))).expect("Problème lors de l'ajout du client.");
        
        info!("Nouveau client connecté : Client {}\n", id);
        
        let mut flux_vers_client = flux.unwrap();
        let flux_vers_serveur: TcpStream = flux_vers_client.try_clone().unwrap();
        /* Thread d'accueil du nouveau client et 
        d'envoi des messages qui lui sont destinés */
        thread::spawn(move ||{
            flux_vers_client.write(format!("Bienvenue sur le serveur! Vous êtes le Client {}\n", id).as_bytes()).unwrap();
            loop {
                let message = au_client.recv().unwrap();
                flux_vers_client.write(message.as_bytes()).unwrap();
                debug!("Message envoyé au client {} : {}\n", id, message);
            }
        });
        
        /* Thread de réception des messages émis par le nouveau client */
        thread::spawn(move ||{
            let mut buffer = BufReader::new(flux_vers_serveur);
            let mut message = String::new();
            for ligne in buffer.lines(){
                match ligne {
                    Ok(ligne) => {
                        debug!("Message reçu du Client {} : {}\nTransmission aux autres en cours...\n", id, ligne);
                        let canal_sortant = vers_serveur.clone();
                        canal_sortant.send(Action::Message(ligne, Participant::new(format!("Client {}", id), vers_clients.clone()))).expect("Problème lors de la transmission du message.");
                    },
                    Err(err) => {
                        error!("Erreur lors de la réception du message du Client {} : {}\n", id, err);
                        break;
                    },
                }
            }
        });
        
    }
    
    Ok(())
    
}




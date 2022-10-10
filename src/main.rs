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
use std::io::{Write, BufRead, BufReader};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc;
use std::env;
use std::thread;

const QUIT_STRING : &str = "!q";

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
impl std::fmt::Debug for Participant {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.nom)
    }
}

/* Actions possibles du master */
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
                            client.messagerie.send(format!("{}: {}\n", auteur.nom, message.clone())).expect("Impossible d'envoyer le message");
                        }
                    }
                },
                //Cas d'un nouveau client
                Action::AjoutParticipant(participant) => {
                    let canal_participant = participant.messagerie.clone();
                    canal_participant.send(format!("Sont connectés: {:?}\n", clients)).expect("Impossible d'envoyer le message");
                    clients.push(participant);
                    
                },
                //Cas d'un client qui se déconnecte
                Action::SuppParticipant(participant) => {
                    clients.retain(|client| client.nom != participant.nom);
                    // participant.messagerie.shutdown().expect("Impossible de fermer la messagerie");
                    debug!("Il reste: {:?}", clients);
                },
            }
        }
    });
    
    /* Gestion des nouvelles connexions */
    for (id, flux) in listener.incoming().enumerate() {
        //Création du canal de communication avec le nouveau client
        let vers_serveur = vers_serveur.clone();
        let (vers_clients, au_client) = mpsc::channel();
        
        //Ajout du participant
        vers_serveur.send(Action::AjoutParticipant(
            Participant::new(format!("Client {}", id), vers_clients.clone()
        ))).expect("Problème lors de l'ajout du client.");

        //Annonce du nouveau participant
        vers_serveur.send(Action::Message(
            format!("Connexion."),
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
                let message = au_client.recv();
                match message {
                    Ok(ref message) => {
                        debug!("Message envoyé au Client {} :'{}'\n", id, message);
                        flux_vers_client.write(message.as_bytes()).unwrap();
                    },
                    Err(_) => {
                        flux_vers_client.write(format!("Vous avez été déconnecté du serveur. Appuyez sur Enter pour quitter.\n").as_bytes()).unwrap();
                        break;
                    },
                }
            }
        });
        
        /* Thread de réception des messages émis par le nouveau client */
        thread::spawn(move ||{
            let buffer = BufReader::new(flux_vers_serveur);
            for message in buffer.lines(){
                match message {
                    Ok(message) => {
                        match message.as_str() {
                            //Cas d'une déconnexion
                            QUIT_STRING => {
                                info!("Client {} déconnecté.", id);
                                vers_serveur.send(Action::SuppParticipant(
                                    Participant::new(format!("Client {}", id), vers_clients.clone()
                                ))).expect("Problème lors de la déconnexion du client.");
                                //on prévient les autres clients
                                vers_serveur.send(Action::Message(
                                    format!("déconnexion."),
                                    Participant::new(format!("Client {}", id), vers_clients.clone()
                                ))).expect("Problème lors de la déconnexion du client.");
                                break;
                            },
                            _ => {
                                //Cas d'un message
                                debug!("Message reçu du Client {} : {}\nTransmission aux autres en cours...\n", id, message);
                                let canal_sortant = vers_serveur.clone();
                                canal_sortant.send(Action::Message(message, Participant::new(format!("Client {}", id), vers_clients.clone()))).expect("Problème lors de la transmission du message.");
                            },
                        }
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




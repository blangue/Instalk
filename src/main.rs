/* Nécessaire au logger personnalisé */
extern crate clap;
extern crate colored;
extern crate error_chain;
#[macro_use]
extern crate log;

mod logger;

//TODO: Finir gestion d'erreur + tout remonter en String avec map_err()
//TODO: Implementer un trait
//TODO: Rendre modulaire

/* Nécessaire au serveur */
use log::LevelFilter;
use std::io::{Write, BufRead, BufReader};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc;
use std::{env};
use std::thread;

const QUIT_STRING : &str = "!q";


/* Code by Bastien LANGUE */

/* Objet Participant */
//Définition du Participant
pub struct Participant {
    nom: String,
    messagerie : mpsc::Sender<String>,
}
fn init_logger() -> Result<(), log::SetLoggerError> {
    log::set_logger(&crate::logger::OurLogger)?;
    log::set_max_level(LevelFilter::Trace);
    Ok(())
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
fn main() -> Result<(), String>{
    /* Configuration du logger amélioré */
    match init_logger(){
        Ok(_) => (),
        Err(_) => println!("Erreur lors de l'initialisation du logger"),
    }
    
    /* Récupération de l'argument PORT */
    let arg: Vec<String> = env::args().collect();
    let port = match get_port(arg){
        Ok(p) => p,
        Err(_) => {
            println!("Erreur lors de la récupération du port");
            return Ok(());
        }
    };
    info!("Démarrage du serveur sur le port {}.", port);
    
    /* Démarrage du serveur et de l'écoute du port */
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).map_err(|e|
        format!("erreur de démarrage du serveur : {}", e.kind())
    )?;
    info!("Serveur démarré sur le port {}, les interlocuteurs peuvent se connecter.", port);
    
    /* ------------- THREAD DU MASTER ------------- */
    let (vers_serveur, au_serveur) =  mpsc::channel();
    /* Canal de communication au serveur */
    let _master = thread::spawn(move || -> Result<(), String> {
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
                            // send_client_sender(&client.nom, client.messagerie, &message);
                            client.messagerie.send(format!("{}: {}\n", auteur.nom, message.clone())).map_err(|e|
                                format!("Impossible d'envoyer le message : {}", e)
                            )?;
                        }
                    }
                },
                //Cas d'un nouveau client
                Action::AjoutParticipant(participant) => {
                    let canal_participant = participant.messagerie.clone();
                    canal_participant.send(format!("Sont connectés: {:?}\n", clients)).map_err(|e|
                        format!("Impossible d'envoyer le message : {}", e)
                    )?;
                    clients.push(participant);
                },
                //Cas d'un client qui se déconnecte
                Action::SuppParticipant(participant) => {
                    clients.retain(|client| client.nom != participant.nom);
                    debug!("Il reste: {:?}", clients);
                },
            }
        }
        Ok(())
    });
    
    /* Gestion des nouvelles connexions */
    for (id, flux) in listener.incoming().enumerate() {
        //Création du canal de communication avec le nouveau client
        let vers_serveur = vers_serveur.clone();
        let (vers_clients, au_client) = mpsc::channel();
        
        //Ajout du participant
        vers_serveur.send(Action::AjoutParticipant(
            Participant::new(format!("Client {}", id), vers_clients.clone()
        ))).map_err(|e|
            format!("Problème lors de l'ajout du client : {}", e)
        )?;
        
        //Annonce du nouveau participant
        vers_serveur.send(Action::Message(
            format!("Connexion."),
            Participant::new(format!("Client {}", id), vers_clients.clone()
        ))).map_err(|e|
            format!("Problème lors du message d'ajout du client : {}", e)
        )?;
        
        info!("Nouveau client connecté : Client {}\n", id);
        
        let mut flux_vers_client = flux.map_err(|e|
            format!("Impossible de récupérer le flux vers le client : {}", e.kind())
        )?;
        let flux_vers_serveur: TcpStream = flux_vers_client.try_clone().map_err(|e|
            format!("Impossible de récupérer le flux vers le client : {}", e.kind())
        )?;
        /* Thread d'accueil du nouveau client et 
        d'envoi des messages qui lui sont destinés */
        thread::spawn(move || -> Result<(), String>{
            welcome_client(&mut flux_vers_client, id)?;
            loop {
                let message = au_client.recv();
                match message {
                    Ok(ref message) => { 
                        send_client_tcp(&format!("{}",id), &mut flux_vers_client, message);
                    },
                    Err(_) => {
                        send_client_tcp(&format!("{}",id), &mut flux_vers_client, &format!("Vous avez été déconnecté du serveur. Appuyez sur Enter pour quitter.\n"));
                        let _ = flux_vers_client.shutdown(Shutdown::Both);
                        break;
                    },
                }
            }
            Ok(())
        });
        
        /* Thread de réception des messages émis par le nouveau client */
        thread::spawn(move || -> Result<(), String>{
            let buffer = BufReader::new(flux_vers_serveur);
            for message in buffer.lines(){
                match message {
                    Ok(message) => {
                        match message.as_str() {
                            //Cas d'une déconnexion
                            QUIT_STRING => {
                                info!("Client {} déconnecté.", id);
                                send_server(&vers_serveur, Action::SuppParticipant(Participant::new(format!("Client {}", id), vers_clients.clone())));
                                //on prévient les autres clients
                                vers_serveur.send(Action::Message(
                                    format!("déconnexion."),
                                    Participant::new(format!("Client {}", id), vers_clients.clone()
                                ))).map_err(|e|
                                    format!("Impossible d'envoyer le message : {}", e)
                                )?;
                                break;
                            },
                            _ => {
                                //Cas d'un message
                                debug!("Message reçu du Client {} : {}\nTransmission aux autres en cours...\n", id, message);
                                let canal_sortant = vers_serveur.clone();
                                canal_sortant.send(Action::Message(message, Participant::new(format!("Client {}", id), vers_clients.clone()))).map_err(|e|
                                    format!("Impossible d'envoyer le message : {}", e)
                                )?
                            },
                        }
                    },
                    Err(err) => {
                        error!("Erreur lors de la réception du message du Client {} : {}\n", id, err);
                        break;
                    },
                }
            }
            Ok(())
        });
        
    }
    
    Ok(())
    
}

fn send_client_tcp(nom: &String, flux: &mut TcpStream, msg: &String) -> () {
    match flux.write(msg.as_bytes()) {
        Ok(_) => {
            debug!("Message envoyé à {} :'{}'\n", nom, msg);
        },
        Err(_) => {
            debug!("Erreur lors de l'envoi du message à {} :'{}'\n", nom, msg);
        }
    };
}

fn send_server(canal_comm: &mpsc::Sender<Action>, action: Action) -> () {
    match canal_comm.send(action) {
        Ok(_) => (),
        Err(_) => {
            debug!("Erreur de communication avec le serveur.");
        }
    };
}

fn welcome_client(flux_vers_client: &mut TcpStream, id: usize) -> Result<(), String> {
    flux_vers_client.write(format!("Bienvenue sur le serveur! Vous êtes le Client {}\n", id).as_bytes()).map_err(|e|
        format!("Erreur de communication avec le client : {}", e.kind())
    )?;
    Ok(())
}

fn get_port(arg: Vec<String>) -> Result<u16, std::num::ParseIntError> {
    let port = arg[1].parse::<u16>()?;
    Ok(port)
}


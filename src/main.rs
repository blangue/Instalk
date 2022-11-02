/* Nécessaire au logger avançé */
extern crate clap;
extern crate colored;
extern crate error_chain;
#[macro_use]
extern crate log;

mod logger;

/* Nécessaire au serveur */
use std::io::{Write, BufRead, BufReader};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc;
use std::env;
use std::thread;

/* Importation des modules */
mod message;
use crate::message::{Message, MessageClient, MessageClientTcp, MessageServer};
mod participant;
use crate::participant::Participant;
mod action;
use crate::action::Action;

const QUIT_STRING : &str = "!q";

/* -------------------- MAIN -------------------- */
fn main() -> Result<(), String>{
    /* Configuration du logger amélioré */
    logger::init_logger()?;
    
    /* Récupération de l'argument PORT */
    let arg: Vec<String> = env::args().collect();
    let port = get_port(arg)?;
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
                            MessageClient::new(
                                client.nom.clone(), 
                                client.messagerie.clone(), 
                                format!("{}: {}\n", auteur.nom, message.clone()
                            )).send()?;
                        }
                    }
                },
                //Cas d'un nouveau client
                Action::AjoutParticipant(participant) => {
                    MessageClient::new(participant.nom.clone(), participant.messagerie.clone(), format!("Sont connectés: {:?}\n", clients)).send()?;
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
        
        // Création du canal de communication avec le nouveau client
        let vers_serveur = vers_serveur.clone();
        let (vers_clients, au_client) = mpsc::channel();
        
        // Ajout du participant
        MessageServer::new(
            vers_serveur.clone(),
            Action::AjoutParticipant(Participant::new(format!("Client {}", id),
            vers_clients.clone()
        ))).send()?;

        // Annonce du nouveau participant
        MessageServer::new(vers_serveur.clone(),
        Action::Message(
            format!("Connexion."),
            Participant::new(format!("Client {}", id),
            vers_clients.clone()
        ))).send().map_err(|e|
            format!("Problème lors du message d'ajout du client :\n{}", e)
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
                let to_client = flux_vers_client.try_clone().map_err(|e|
                    format!("Impossible de récupérer le flux vers le client : {}", e.kind())
                )?;
                let mut order = MessageClientTcp::new(format!("{}",id), to_client, String::new());
                match message {
                    Ok(message) => { 
                        order.set_message(message);
                        order.send()?;
                    },
                    Err(_) => {
                        order.set_message(format!("Vous avez été déconnecté du serveur. Appuyez sur Enter pour quitter.\n"));
                        order.send()?;
                        flux_vers_client.shutdown(Shutdown::Both).map_err(|e| 
                            format!("Echec de la fermeture du canal de communication : {}\n", e.kind())
                        )?;
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
                                MessageServer::new(vers_serveur.clone(), Action::SuppParticipant(
                                    Participant::new(format!("Client {}", id),
                                    vers_clients.clone()
                                ))).send()?;
                                //On prévient les autres clients
                                MessageServer::new(vers_serveur.clone(), Action::Message(
                                    format!("déconnexion."),
                                    Participant::new(format!("Client {}", id), vers_clients.clone()
                                ))).send()?;
                                break;
                            },
                            _ => {
                                //Cas d'un message
                                debug!("Message reçu du Client {} : {}\nTransmission aux autres en cours...\n", id, message);
                                let canal_sortant = vers_serveur.clone();
                                MessageServer::new(canal_sortant, Action::Message(
                                    message, 
                                    Participant::new(format!("Client {}", id), 
                                    vers_clients.clone()
                                ))).send()?;
                            },
                        }
                    }, 
                    Err(e) => {
                        format!("Erreur lors de la réception du message du Client {} : {}\n", id, e.kind());
                    }
                }
            }
            Ok(())
        });
        
    }
    
    Ok(())
    
}

fn welcome_client(flux_vers_client: &mut TcpStream, id: usize) -> Result<(), String> {
    flux_vers_client.write(format!("Bienvenue sur le serveur! Vous êtes le Client {}\n", id).as_bytes()).map_err(|e|
        format!("Erreur de communication avec le client : {}", e.kind())
    )?;
    Ok(())
}

fn get_port(arg: Vec<String>) -> Result<u16, String> {
    if arg.len() != 2 {return Err(format!("Merci d'entrer un numéro de port en argument"));}
    let port = arg[1].parse::<u16>().map_err(|e|
        format!("Merci d'entrer un numéro entier de port : {}", e)
    )?;
    Ok(port)
}


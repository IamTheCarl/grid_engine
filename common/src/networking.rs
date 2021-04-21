// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Networking protocols and utilities.

use anyhow::{anyhow, Result};
// use laminar::{Packet, Socket, SocketEvent};
use rsa::{PublicKeyParts, RSAPrivateKey, RSAPublicKey};

use std::net::{IpAddr, TcpListener, TcpStream, UdpSocket};

struct ServerConnectionManager {
    private_key: RSAPrivateKey,
    unvalidated_clients: Vec<UnvalidatedClient>,
    validated_clients: Vec<ValidatedClient>,
    banned_addresses: Vec<IpAddr>,
    // banned_keys: Vec<>
}

struct UnvalidatedClient {
    tcp_connection: TcpStream,
}

struct ValidatedClient {
    public_key: RSAPublicKey,
    tcp_connection: TcpStream,
    udp_connection: UdpSocket,
}

struct ServerConnection {
    our_private_key: RSAPrivateKey,
    server_public_key: RSAPublicKey,
}

use std::io::{Listener, Acceptor};
use std::io::net::tcp::{TcpListener, TcpStream};
use std::io::net::ip::{SocketAddr, Ipv4Addr};
use std::comm::{Select};


fn client_input_handler(common_channel: Chan<~str>, mut client_stream: TcpStream) {
  assert!(client_stream.write_str("welcome to the chat!\n").is_ok());

  let mut line_buffer = ~[];

  loop {
    let byte = client_stream.read_byte().unwrap();
    line_buffer.push(byte);
    if byte == 0x0A {
      let line = std::str::from_utf8(line_buffer).unwrap().into_owned();
      common_channel.send(line);
      line_buffer = ~[];
    }
  }
}

fn client_output_handler(messages_port: Port<~str>, mut client_stream: TcpStream) {
  loop {
    assert!(client_stream.write_str(messages_port.recv()).is_ok());
  }
}

fn multiplexer(client_stream_port: Port<Chan<~str>>, messages_from_clients: Port<~str>) {
  let mut client_streams = ~[];

  let port_set = Select::new();
  let mut client_stream_port_handle = port_set.handle(&client_stream_port);
  let mut messages_from_clients_handle = port_set.handle(&messages_from_clients);

  unsafe {
    client_stream_port_handle.add();
    messages_from_clients_handle.add();
  }

  loop {
    let handle_id = port_set.wait();

    if handle_id == client_stream_port_handle.id() {
      client_streams.push(client_stream_port.recv());
    } else if handle_id == messages_from_clients_handle.id() {
      let message = messages_from_clients_handle.recv();
      for client_stream in client_streams.iter() {
        client_stream.send(message.clone());
      }
    }
  }
}


fn main() {
  let mut acceptor = TcpListener::bind(SocketAddr {
                                           ip: Ipv4Addr(127, 0, 0, 1),
                                           port: 9123
                                       }).listen();
  println!("listening started, ready to accept");

  let (messages_from_clients, common_chan) = Chan::new();
  let (client_streams_port, client_streams_chan) = Chan::new();

  spawn(proc() {
    multiplexer(client_streams_port, messages_from_clients);
  });

  loop {
    let client_stream = acceptor.accept().unwrap();
    let output_client_stream = client_stream.clone();
    let local_common_channel = common_chan.clone();
    let (port_to_client, chan_to_client) = Chan::new();

    client_streams_chan.send(chan_to_client);

    spawn(proc() {
      client_input_handler(local_common_channel, client_stream);
    });

    spawn(proc() {
      client_output_handler(port_to_client, output_client_stream);
    });
  }
}

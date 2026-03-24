const net = require('net');

const pipeName = '\\\\.\\pipe\\egui_mcp_client';

const client = net.createConnection(pipeName, () => {
    console.log('Connected to pipe');
    
    // Send ping
    const message = {type: "ping"};
    client.write(JSON.stringify(message) + '\n');
});

client.on('data', (data) => {
    console.log('Response:', data.toString());
    client.end();
});

client.on('error', (err) => {
    console.error('Error:', err);
});

client.on('end', () => {
    console.log('Disconnected');
});

import subprocess
import time
import json
import sys
import os
import threading

def read_output(stream, prefix):
    for line in iter(stream.readline, b''):
        print(f"[{prefix}] {line.decode().strip()}")

def verify_interaction():
    print("Building binaries...")
    subprocess.run(["cargo", "build", "--bin", "demo-app-win", "--bin", "egui-mcp-server-win"], check=True)

    print("Starting Demo App...")
    # Start demo app in background
    demo_app = subprocess.Popen(
        ["target/debug/demo-app-win.exe"],
        cwd=".",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    
    # Give it time to initialize GUI
    time.sleep(5)
    
    print("Starting MCP Server...")
    # Start server with stdin pipe
    server = subprocess.Popen(
        ["target/debug/egui-mcp-server-win.exe"],
        cwd=".",
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    
    # Thread to read server stderr
    threading.Thread(target=read_output, args=(server.stderr, "SERVER_LOG"), daemon=True).start()
    
    time.sleep(2) # Wait for server to be ready
    
    id_counter = 1
    
    def send_request(method, params=None):
        nonlocal id_counter
        req = {
            "jsonrpc": "2.0",
            "id": id_counter,
            "method": method,
            "params": params or {}
        }
        id_counter += 1
        msg = json.dumps(req) + "\n"
        print(f"Sending: {msg.strip()}")
        server.stdin.write(msg.encode())
        server.stdin.flush()
        
        # Read response (blocking line read)
        response_line = server.stdout.readline().decode()
        print(f"Response: {response_line.strip()}")
        return json.loads(response_line) if response_line else None

    try:
        # 1. Initialize
        send_request("initialize", {"clientInfo": {"name": "verifier", "version": "1.0"}, "capabilities": {}})
        
        # 2. Set Slider to 100
        # Check if we use 'set_slider_value' handler which finds a slider and sets it
        # Or if we should do it manually via find -> click
        print("\n--- Testing Slider ---")
        # Trying the convenience method I saw in lib.rs
        send_request("set_slider_value", {"value": 100.0})
        
        time.sleep(1)
        
        # 3. Toggle Checkbox
        print("\n--- Testing Checkbox ---")
        # Find checkbox by name
        res = send_request("find_by_label", {"label": "Checkbox"})
        if res and res.get("result") and len(res["result"]) > 0:
            checkbox = res["result"][0]
            print(f"Found checkbox: {checkbox['id']}")
            
            # Click it
            # We can use click_element(id) or click_at
            # Let's try click_element first
            send_request("click_element", {"element_id": checkbox["id"]})
        else:
            print("Failed to find 'Checkbox'")
            
        time.sleep(2)
        
    finally:
        print("Terminating processes...")
        server.terminate()
        demo_app.terminate()

if __name__ == "__main__":
    verify_interaction()

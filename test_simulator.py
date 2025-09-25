#!/usr/bin/env python3
"""Quick test of KNX simulator"""

import socket
import struct

def build_connect_request():
    """Build a CONNECT_REQUEST frame"""
    # Header: len=6, version=0x10, service=0x0205, total_len=26
    header = struct.pack('>BBHH', 0x06, 0x10, 0x0205, 26)

    # Control HPAI (8 bytes): len=8, protocol=UDP(0x01), ip=0.0.0.0, port=0
    hpai_control = struct.pack('>BB4sH', 0x08, 0x01, bytes([0, 0, 0, 0]), 0)

    # Data HPAI (8 bytes)
    hpai_data = struct.pack('>BB4sH', 0x08, 0x01, bytes([0, 0, 0, 0]), 0)

    # CRI (Connection Request Information) - 4 bytes
    # len=4, connection_type=TUNNEL_CONNECTION(0x04), knx_layer=TUNNEL_LINKLAYER(0x02), reserved=0
    cri = struct.pack('>BBH', 0x04, 0x04, 0x0200)

    return header + hpai_control + hpai_data + cri

def parse_connect_response(data):
    """Parse CONNECT_RESPONSE"""
    if len(data) < 8:
        return None

    header_len, version, service_type, total_len = struct.unpack('>BBHH', data[:6])
    channel_id, status = struct.unpack('BB', data[6:8])

    return {
        'service_type': service_type,
        'channel_id': channel_id,
        'status': status
    }

def test_simulator():
    """Test the simulator with a CONNECT_REQUEST"""
    print("Testing KNX Simulator...")

    # Create UDP socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(2.0)

    try:
        # Build and send CONNECT_REQUEST
        connect_req = build_connect_request()
        print(f"Sending CONNECT_REQUEST ({len(connect_req)} bytes)...")
        sock.sendto(connect_req, ('127.0.0.1', 3671))

        # Receive CONNECT_RESPONSE
        data, addr = sock.recvfrom(1024)
        print(f"Received response ({len(data)} bytes) from {addr}")

        # Parse response
        response = parse_connect_response(data)
        if response:
            print(f"  Service Type: 0x{response['service_type']:04X}")
            print(f"  Channel ID: {response['channel_id']}")
            print(f"  Status: {response['status']} {'(OK)' if response['status'] == 0 else '(ERROR)'}")

            if response['service_type'] == 0x0206 and response['status'] == 0:
                print("\n✅ SUCCESS! Simulator is working correctly!")
                return True
        else:
            print("❌ Failed to parse response")
            return False

    except socket.timeout:
        print("❌ Timeout - simulator not responding")
        return False
    except Exception as e:
        print(f"❌ Error: {e}")
        return False
    finally:
        sock.close()

if __name__ == '__main__':
    test_simulator()

#!/usr/bin/env python3
"""
Simple KNXnet/IP Gateway Simulator for testing knx-rs

This simulates a basic KNXnet/IP gateway that responds to:
- CONNECT_REQUEST/RESPONSE
- DISCONNECT_REQUEST/RESPONSE
- TUNNELING_REQUEST/ACK/INDICATION
- CONNECTIONSTATE_REQUEST/RESPONSE
- SEARCH_REQUEST/RESPONSE (added)

Usage:
    python3 knx_simulator.py [--port 3671] [--verbose]
"""

import socket
import struct
import argparse
import time
from datetime import datetime

# KNXnet/IP Service Type Identifiers
SERVICE_SEARCH_REQUEST = 0x0201
SERVICE_SEARCH_RESPONSE = 0x0202
SERVICE_CONNECT_REQUEST = 0x0205
SERVICE_CONNECT_RESPONSE = 0x0206
SERVICE_CONNECTIONSTATE_REQUEST = 0x0207
SERVICE_CONNECTIONSTATE_RESPONSE = 0x0208
SERVICE_DISCONNECT_REQUEST = 0x0209
SERVICE_DISCONNECT_RESPONSE = 0x020A
SERVICE_TUNNELING_REQUEST = 0x0420
SERVICE_TUNNELING_ACK = 0x0421
SERVICE_TUNNELING_INDICATION = 0x0420  # for gateway->client indication

# Status codes
STATUS_OK = 0x00
STATUS_NO_ERROR = 0x00

class KNXSimulator:
    def __init__(self, port=3671, verbose=False):
        self.port = port
        self.verbose = verbose
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        # allow reuse of address when restarting
        try:
            self.sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        except Exception:
            pass
        self.sock.bind(('0.0.0.0', port))
        self.channels = {}  # channel_id -> (client_addr, sequence_counter)
        self.next_channel = 1

    def log(self, msg):
        if self.verbose:
            timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
            print(f"[{timestamp}] {msg}")

    def parse_header(self, data):
        """Parse KNXnet/IP header.
        Header layout used here: header_len (1), version (1), service_type (2), total_length (2)
        The function returns a dict with parsed fields and body bytes (data after header).
        """
        if len(data) < 6:
            return None
        try:
            header_len, protocol_version, service_type, total_len = struct.unpack('>BBHH', data[:6])
        except Exception:
            return None
        body = data[6:6 + (total_len - 6)] if total_len >= 6 else data[6:]
        return {
            'header_len': header_len,
            'version': protocol_version,
            'service_type': service_type,
            'total_len': total_len,
            'body': body
        }

    def build_header(self, service_type, body_len):
        """Build KNXnet/IP header using same layout used in parse_header."""
        total_len = 6 + body_len
        return struct.pack('>BBHH', 0x06, 0x10, service_type, total_len)

    def handle_search_request(self, data, client_addr):
        """Handle SEARCH Request and send a SearchResponse to the HPAI declared in the request.
        The SearchRequest payload is expected to contain at least one HPAI (8 bytes):
        HPAI: len (1), protocol (1=IPv4 UDP), IPv4(4), port(2)
        If present, we send the SearchResponse to that HPAI. Otherwise, respond to the source.
        """
        self.log(f"SEARCH_REQUEST from {client_addr}")
        target_ip, target_port = client_addr

        # parse first HPAI if present
        if len(data) >= 8:
            try:
                hpai_len = data[0]
                proto = data[1]
                if hpai_len == 8 and proto == 1:
                    ip_bytes = data[2:6]
                    port = struct.unpack('!H', data[6:8])[0]
                    target_ip = socket.inet_ntoa(ip_bytes)
                    target_port = port
            except Exception:
                pass

        # compute server IP to advertise (choose interface used to reach target)
        try:
            tmp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            tmp.connect((target_ip, target_port))
            server_ip = tmp.getsockname()[0]
            tmp.close()
        except Exception:
            server_ip = '0.0.0.0'

        # build body: HPAI of server (len=8, proto=1, ipv4, port)
        body = struct.pack('>BB4sH', 0x08, 0x01, socket.inet_aton(server_ip), self.port)
        header = self.build_header(SERVICE_SEARCH_RESPONSE, len(body))
        response = header + body

        try:
            self.sock.sendto(response, (target_ip, target_port))
            self.log(f"  → SEARCH_RESPONSE sent to {target_ip}:{target_port} advertising {server_ip}:{self.port}")
        except Exception as e:
            self.log(f"  → Failed to send SEARCH_RESPONSE: {e}")

    def handle_connect_request(self, data, client_addr):
        self.log(f"CONNECT_REQUEST from {client_addr}")
        channel_id = self.next_channel
        self.channels[channel_id] = (client_addr, 0)
        self.next_channel += 1

        # Build CONNECT_RESPONSE body
        body = struct.pack('BB', channel_id, STATUS_OK)
        # HPAI (control endpoint) + CRD
        body += struct.pack('>BB4sH', 0x08, 0x01, b'\x00\x00\x00\x00', 0)
        body += struct.pack('>BBH', 0x04, 0x04, 0x0200)

        header = self.build_header(SERVICE_CONNECT_RESPONSE, len(body))
        response = header + body
        self.log(f"  → CONNECT_RESPONSE: channel={channel_id}")
        return response

    def handle_disconnect_request(self, data, client_addr):
        if len(data) < 2:
            return None
        channel_id, status = struct.unpack('BB', data[:2])
        self.log(f"DISCONNECT_REQUEST: channel={channel_id}")
        if channel_id in self.channels:
            del self.channels[channel_id]
        body = struct.pack('BB', channel_id, STATUS_OK)
        header = self.build_header(SERVICE_DISCONNECT_RESPONSE, len(body))
        return header + body

    def handle_tunneling_request(self, data, client_addr):
        if len(data) < 4:
            return None
        conn_header = data[:4]
        header_len, channel_id, sequence, reserved = struct.unpack('BBBB', conn_header)
        cemi_data = data[4:]
        self.log(f"TUNNELING_REQUEST: channel={channel_id}, seq={sequence}, cemi_len={len(cemi_data)}")

        # build ACK (connection header + status)
        body = conn_header + struct.pack('B', STATUS_OK)
        header = self.build_header(SERVICE_TUNNELING_ACK, len(body))
        response = header + body
        self.log(f"  → TUNNELING_ACK: seq={sequence}")

        # Send TUNNELING_INDICATION only for GroupWrite commands (realistic gateway behavior)
        # Real KNX gateways echo GroupWrite commands back immediately (within milliseconds)
        # GroupRead commands don't get echoed - they wait for a response from another device
        if len(cemi_data) > 0 and self.is_group_write(cemi_data):
            self.send_tunneling_indication(channel_id, cemi_data)
            self.log(f"  → TUNNELING_INDICATION sent (echo of GroupWrite)")

        return response

    def is_group_write(self, cemi_data):
        """Check if cEMI frame is a GroupWrite command.

        cEMI format:
        - Byte 0: Message code (0x11 = L_Data.req, 0x29 = L_Data.ind)
        - Byte 1: Add info length
        - Bytes 2+: Control fields, addresses, NPDU

        APCI (Application Protocol Control Information) is in the TPCI/APCI byte:
        - 0x00 = GroupValue_Read
        - 0x40 = GroupValue_Response
        - 0x80 = GroupValue_Write
        """
        if len(cemi_data) < 10:  # Minimum valid cEMI frame
            self.log(f"  [DEBUG] cEMI too short: {len(cemi_data)} bytes (need >= 10)")
            return False

        try:
            # Debug: print full cEMI frame
            self.log(f"  [DEBUG] cEMI hex: {cemi_data.hex()}")

            # Parse cEMI structure
            msg_code = cemi_data[0]
            add_info_len = cemi_data[1]

            self.log(f"  [DEBUG] Message code: 0x{msg_code:02X}, Add info len: {add_info_len}")

            # Calculate TPCI/APCI position accounting for additional info
            # Base structure: msg_code(1) + add_info_len(1) + add_info(N) + ctrl1(1) + ctrl2(1) + src(2) + dst(2) + npdu_len(1) + tpci_apci(1)
            tpci_apci_pos = 2 + add_info_len + 1 + 1 + 2 + 2 + 1  # = 9 + add_info_len

            if len(cemi_data) <= tpci_apci_pos:
                self.log(f"  [DEBUG] cEMI too short for TPCI/APCI at pos {tpci_apci_pos}")
                return False

            # In KNX cEMI, the APCI is split across two bytes:
            # Byte 9 (tpci_apci_pos): TPCI + upper 2 bits of APCI
            # Byte 10 (tpci_apci_pos + 1): lower 4 bits of APCI + data
            #
            # For GroupValue commands:
            # - GroupValue_Read: APCI = 0x0000
            # - GroupValue_Response: APCI = 0x0040
            # - GroupValue_Write: APCI = 0x0080
            #
            # The APCI upper bits are in byte 9, bits 0-1 (mask 0x03)
            tpci_apci_byte = cemi_data[tpci_apci_pos]
            apci_upper = (tpci_apci_byte & 0x03) << 6  # Get bits 0-1 and shift to position

            # Next byte contains lower APCI bits and data
            if len(cemi_data) <= tpci_apci_pos + 1:
                self.log(f"  [DEBUG] cEMI missing APCI+data byte")
                return False

            apci_data_byte = cemi_data[tpci_apci_pos + 1]
            apci_lower = apci_data_byte & 0xC0  # Upper 2 bits of data byte

            apci = apci_upper | apci_lower

            self.log(f"  [DEBUG] TPCI byte at pos {tpci_apci_pos}: 0x{tpci_apci_byte:02X}")
            self.log(f"  [DEBUG] APCI+data byte at pos {tpci_apci_pos + 1}: 0x{apci_data_byte:02X}")
            self.log(f"  [DEBUG] Combined APCI: 0x{apci:02X} (upper: 0x{apci_upper:02X}, lower: 0x{apci_lower:02X})")

            is_write = apci == 0x80
            self.log(f"  [DEBUG] Is GroupWrite? {is_write}")

            return is_write
        except Exception as e:
            self.log(f"  [DEBUG] Exception in is_group_write: {e}")
            return False

    def handle_connectionstate_request(self, data, client_addr):
        if len(data) < 2:
            return None
        channel_id, reserved = struct.unpack('BB', data[:2])
        self.log(f"CONNECTIONSTATE_REQUEST: channel={channel_id}")
        body = struct.pack('BB', channel_id, STATUS_OK)
        header = self.build_header(SERVICE_CONNECTIONSTATE_RESPONSE, len(body))
        return header + body

    def build_cemi_group_write(self, group_addr, value_bool):
        cemi = bytearray()
        cemi.append(0x29)  # L_Data.ind
        cemi.append(0x00)  # additional info len
        cemi.append(0xBC)  # control field 1
        cemi.append(0xE0)  # control field 2
        cemi.extend([0x11, 0xFA])  # source 1.1.250
        cemi.extend(struct.pack('>H', group_addr))
        cemi.append(0x01)  # NPDU length
        cemi.append(0x00)  # TPCI/APCI
        apci_data = 0x81 if value_bool else 0x80
        cemi.append(apci_data)
        return bytes(cemi)

    def send_tunneling_indication(self, channel_id, cemi_data):
        if channel_id not in self.channels:
            return
        client_addr, sequence = self.channels[channel_id]
        conn_header = struct.pack('BBBB', 0x04, channel_id, sequence, 0x00)
        body = conn_header + cemi_data
        header = self.build_header(SERVICE_TUNNELING_INDICATION, len(body))
        frame = header + body
        self.sock.sendto(frame, client_addr)
        self.channels[channel_id] = (client_addr, (sequence + 1) % 256)
        self.log(f"  → TUNNELING_INDICATION: channel={channel_id}, seq={sequence}")

    def run(self):
        print(f"=== KNX Gateway Simulator ===")
        print(f"Listening on 0.0.0.0:{self.port}")
        print(f"Gateway address: 1.1.250")
        print(f"Client addresses: 1.1.128 - 1.1.135")
        print(f"Press Ctrl+C to stop\n")

        try:
            while True:
                data, client_addr = self.sock.recvfrom(2048)
                print(f"\n[RAW] Received {len(data)} bytes from {client_addr}")
                print(f"      Hex: {data.hex()}")

                frame = self.parse_header(data)
                if not frame:
                    self.log(f"Invalid frame from {client_addr}")
                    print("      ERROR: Failed to parse header")
                    continue

                response = None
                service_type = frame['service_type']
                body = frame['body']

                if service_type == SERVICE_CONNECT_REQUEST:
                    response = self.handle_connect_request(body, client_addr)
                elif service_type == SERVICE_DISCONNECT_REQUEST:
                    response = self.handle_disconnect_request(body, client_addr)
                elif service_type == SERVICE_TUNNELING_REQUEST:
                    response = self.handle_tunneling_request(body, client_addr)
                elif service_type == SERVICE_CONNECTIONSTATE_REQUEST:
                    response = self.handle_connectionstate_request(body, client_addr)
                elif service_type == SERVICE_SEARCH_REQUEST:
                    # handle_search_request sends directly to HPAI; no response object returned
                    self.handle_search_request(body, client_addr)
                elif service_type == SERVICE_TUNNELING_ACK:
                    # Client acknowledges our TUNNELING_INDICATION - this is expected
                    if len(body) >= 4:
                        header_len, channel_id, sequence, status = struct.unpack('BBBB', body[:4])
                        self.log(f"TUNNELING_ACK received: channel={channel_id}, seq={sequence}, status={status}")
                    else:
                        self.log(f"TUNNELING_ACK received (short frame)")
                else:
                    self.log(f"Unknown service type: 0x{service_type:04X}")

                if response:
                    self.sock.sendto(response, client_addr)

        except KeyboardInterrupt:
            print("\n\nShutting down...")
        finally:
            self.sock.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='KNXnet/IP Gateway Simulator')
    parser.add_argument('--port', type=int, default=3671, help='UDP port (default: 3671)')
    parser.add_argument('-v', '--verbose', action='store_true', help='Verbose logging')
    args = parser.parse_args()

    simulator = KNXSimulator(port=args.port, verbose=args.verbose)
    simulator.run()

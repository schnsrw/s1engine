#!/usr/bin/env python3
"""Simple HTTP server for the s1engine demo with correct WASM MIME types."""
import http.server
import os
import sys

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8080

os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'demo'))

class WasmHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        '.wasm': 'application/wasm',
        '.js': 'application/javascript',
        '.mjs': 'application/javascript',
    }

    def end_headers(self):
        self.send_header('Cache-Control', 'no-cache')
        self.send_header('Access-Control-Allow-Origin', '*')
        super().end_headers()

print(f'Serving demo at http://localhost:{PORT}')
print('Press Ctrl+C to stop.\n')
http.server.HTTPServer(('', PORT), WasmHandler).serve_forever()

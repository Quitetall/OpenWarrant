#!/usr/bin/python3
"""`ask-customer "question"`: one question to the customer, over the socket the harness bound here."""
import socket
import sys

question = " ".join(sys.argv[1:]).strip()
if not question:
    sys.exit('usage: ask-customer "your question"')
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(30)
s.connect("/opt/study/customer.sock")
s.sendall(question.replace("\n", " ").encode() + b"\n")
reply = b""
while True:
    chunk = s.recv(4096)
    if not chunk:
        break
    reply += chunk
sys.stdout.write(reply.decode("utf-8", "replace"))

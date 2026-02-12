# QF Network Mainnet Node Deployment Guide

This document provides a technical walkthrough for deploying a Substrate-based node for the **QF Network** (`qf-mainnet`). This setup uses the official container image and is configured for high-performance RPC access with specific pruning and database settings.

## 1. System Requirements

Linux OS on AMD64 (Epyc recommended) hardware. The requirements below are considering high performance CPU optimized VM setups. With dedicated hardware, the CPU requirements could be reduced.

It's recommended to keep the OS and data volumes separate, for best performance and easier block storage resizing on most Cloud providers.

| Node Type               | CPU (Min) | CPU (Rec) | RAM (Min)                        | RAM (Rec)                         | OS Drive  | Data Drive (Min) | Data Drive (Rec) |
| ----------------------- | --------- | --------- | -------------------------------- | --------------------------------- | --------- | ---------------- | ---------------- |
| Validator               | 4 cores   | 8 cores   | 16 GB                            | 32 GB                             | 30 GB SSD | 100 GB SSD       | 150 GB SSD       |
| Archive                 | 2 cores   | 4 cores   | 8 GB                             | 16 GB                             | 30 GB SSD | 300 GB SSD       | 500 GB SSD       |
| Pruned RPC (36k blocks) | 2 cores   | 4 cores   | 4 GB (ParityDB) / 8 GB (RocksDB) | 8 GB (ParityDB) / 16 GB (RocksDB) | 30 GB SSD | 120 GB SSD       | 120 GB SSD       |

Note: the RAM requirements for RocksDB databases are higher due to the high amount of blocks we need to retain ~1h of history.

## 2. Deployment via Docker/Podman Compose

Docker/Podman Compose is the recommended method for validator and archive nodes in production environments as it ensures consistent configuration and easy service management. Pruned RPC nodes would work better with Kubernetes as it allows auto healing and scaling if set up properly.

It's recommended to create a separate user and run the container with Podman to run it rootless. Make sure to enable lingering for that user, so the container keeps running when the user logs out. For convenience sake, the method below describes a Docker Compose setup on a Debian Linux based system as the root user.

### RPC node Setup Steps

1. Create a directory for the node data on the data drive: `mkdir -p /mnt/qf-node-data && cd /mnt/qf-node-data`.

2. Create a `compose.yml` file and paste the following:

```yaml
services:
  qf-node:
    image: docker.io/theqfnetwork/qf-node:v0.1.28
    container_name: qf-rpc-node
    network_mode: host
    restart: always
    volumes:
      - /mnt/qf-node-data:/data
    command:
      - qf-node
      - --name=qf-mainnet-rpc-node
      - --chain=qf-mainnet
      - --port=30333
      - --rpc-port=9944
      - --rpc-cors=all
      - --prometheus-port=9615
      - --base-path=/data
      - --database=paritydb # Or remove this flag to use the default rocksdb
      - --state-pruning=archive # For an archive node. For a pruned node use `- --state-pruning=36000`.
      - --blocks-pruning=archive # For an archive node. For a pruned node use `- --blocks-pruning=36000`.
```

3. Set up your firewall to allow p2p connectivity and secure RPC connections. With Nftables, `/etc/nftables.conf` should look something like this:

```
#!/usr/sbin/nft -f

flush ruleset

table inet filter {
  chain input {
    type filter hook input priority 0; policy drop;

    ct state invalid drop
    ct state {established, related} accept

    iif "lo" accept
    ip protocol icmp accept
    meta l4proto ipv6-icmp accept

    tcp dport 22 log prefix "SSH_CONNECT: " accept
    tcp dport 30333 counter accept comment "P2P networking"
    tcp dport 443 ct state { new, established } accept comment "HTTPS WebSocket"
    tcp dport 80 jump certbot
  }

  chain forward {
    type filter hook forward priority 0; policy drop;
  }

  chain output {
    type filter hook output priority 0; policy accept;
  }

  # Used by Certbot to temporarily open port 80 for the HTTP-01 challenge
  chain certbot {}
}
```

If you don't have Nftables installed, you can install with `sudo apt update && sudo apt install nftables`. Note: disable iptables before you enable Nftables with `sudo systemctl enable --now nftables`.

To disable iptables, run these but be sure the migrate existing rules to Nftables first:

```bash
# On Ubuntu
sudo ufw disable; sudo systemctl stop ufw; sudo systemctl disable ufw

sudo systemctl stop iptables; sudo systemctl disable iptables; sudo systemctl stop ip6tables; sudo systemctl disable ip6tables

# Reset the rules too
sudo iptables -F; sudo iptables -X; sudo ip6tables -F; sudo ip6tables -X

# Verify with:
sudo iptables -L -n

# And that your Nftables rules are still intact:
sudo nft list ruleset
```

4. Then apply it with `sudo systemctl restart nftables`.


5. Now you're ready to start the node. Before you continue, make sure your user is part of the `docker` group: `sudo usermod -aG docker "$USER"` and relogin. Then run `docker compose up -d` in the directory where the compose.yml file is located. Enable the Docker daemon to automatically start after a server reboot with `sudo systemctl enable docker.service`. The node will now start syncing blocks.


6. To provide secure websocket connections, install Nginx and certbot with `sudo apt install nginx certbot` and activate nginx: `sudo systemctl enable --now nginx`. You also need a domain name that's pointing to your server's IP. I'll use YOUR_DOMAIN_NAME as a placeholder in these instructions.


7. Run the followings commands to prepare for the certificate generation:

```bash
sudo tee /etc/nginx/sites-available/certbot.conf >/dev/null <<'EOF'
server {
    listen 80;
    listen [::]:80;
    server_name YOUR_DOMAIN_NAME;

    location ^~ /.well-known/acme-challenge/ {
        root /var/www/letsencrypt;
        default_type "text/plain";
        try_files $uri =404;
    }

    location / {
        return 301 https://$host$request_uri;
    }
}
EOF

sudo mkdir -p /var/www/letsencrypt/.well-known/acme-challenge

sudo ln -sf /etc/nginx/sites-available/certbot.conf /etc/nginx/sites-enabled/

sudo nginx -t && sudo systemctl reload nginx

```

8. Now we're going to set up the Nginx site for the websocket connections:

```bash
sudo tee /etc/nginx/sites-available/rpc.conf >/dev/null <<'EOF'
server {
    listen 443 ssl;
    listen [::]:443 ssl;
    server_name YOUR_DOMAIN_NAME;

    http2 on;

    # SSL certificates (managed by certbot)
    ssl_certificate /etc/letsencrypt/live/YOUR_DOMAIN_NAME/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/YOUR_DOMAIN_NAME/privkey.pem;

    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;

    location / {
        proxy_pass http://127.0.0.1:9944;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        proxy_connect_timeout 5s;
        proxy_send_timeout 1h;
        proxy_read_timeout 1h;
    }
}
EOF
```

9. Generate a TLS certificate with:

```bash
sudo certbot certonly --webroot -w /var/www/letsencrypt -d YOUR_DOMAIN_NAME \
  --pre-hook "nft add rule inet filter certbot tcp dport 80 accept" \
  --post-hook "nft flush chain inet filter certbot" \
  --deploy-hook "systemctl reload nginx"
```

This will also create an autorenewal hook with these options.

---

 ====== Now your RPC node has been set up. ======

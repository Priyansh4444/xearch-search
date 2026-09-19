# VM services

The VM runs the static frontend and raw capture receiver and, after a coordinated
cutover from the Mac, the outbound production worker as user-level systemd
services. Convex stays on the existing hosted production deployment. These
services do not host a development server, Convex, Elasticsearch, or another
search engine.

Install the unit files from the repository and create the private log directory:

```sh
install -d -m 700 ~/.config/systemd/user .local-captures/logs
install -m 600 deploy/systemd/xearch-capture.service ~/.config/systemd/user/
install -m 600 deploy/systemd/xearch-production-worker.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now xearch-capture.service
```

Do not enable or start `xearch-production-worker.service` until the Mac worker is
confirmed stopped and any final capture sync is complete. At cutover:

```sh
systemctl --user enable --now xearch-production-worker.service
```

Build the frontend against the existing production Convex deployment without
editing the transferred environment files, then install its service:

```sh
VITE_CONVEX_URL=https://utmost-kudu-321.convex.cloud bun run build
install -d -m 700 .local-hosting/logs
install -m 600 deploy/systemd/xearch-frontend.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now xearch-frontend.service
```

The frontend listens on port 8080 for the exe.dev HTTPS proxy. Configure the
documented private proxy with `ssh exe.dev share port exp-xearch 8080`. Do not
make the proxy public without an explicit launch decision. The resulting private
URL is `https://exp-xearch.exe.xyz/`.

All services restart automatically. The receiver listens only on
`127.0.0.1:4319`. Worker/capture logs are written beneath
`.local-captures/logs/`; nginx logs are under `.local-hosting/logs/`. These
directories and their files must remain owner-only. Inspect status without
printing credentials:

```sh
systemctl --user status xearch-capture.service
systemctl --user status xearch-production-worker.service
systemctl --user status xearch-frontend.service
curl --fail --silent http://127.0.0.1:4319/health
curl --fail --silent http://127.0.0.1:8080/
ss -ltnp 'sport = :4319'
```

The committed units are specific to the `exedev` checkout path on this VM. If the
repository or Bun executable moves, update both the committed and installed
units together.

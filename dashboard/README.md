## Simple dashboard for local environment

This directory provides a simple data visualization tool for those who want to check in their local environment or do not have a visualization environment.

This visualization tool will use Prometheus for data storage. You can also use your own Prometheus, but to support OTLP, you need to enable OTLP with the experimental flag or use version 3 or higher.

![dashboard](./dashboard.png)

## Security

This dashboard is intended for use during development or for simple use, so it is not recommended for use in a production environment.
This dashboard does not have authentication or restrictions, so please use Grafana or Perses, etc. from a security and availability standpoint in a production environment.

## Notes

This server retrieves and renders data from the server specified by the `--datasource-url` argument or the `HB_DASHBOARD_DATASOURCE_URL` environment variable through the Prometheus query API at startup.
The browser does not directly access Prometheus; instead, it goes through the web server process. Therefore, if you start it in a container, make sure the container can access Prometheus.

## How to run

```bash
$ docker run --rm -it -p 8080:8080 ghcr.io/watawuwu/hb-dashboard
$ open http://localhost:8080

# if you want to use your machine's Prometheus or external Prometheus, you can specify the datasource-url argument or environment variable.
$ docker run --rm -it -p 8080:8080 ghcr.io/watawuwu/hb-dashboard --datasource-url http://host.docker.internal:9090
$ open http://localhost:8080
```

```bash
$ docker run --rm -it  ghcr.io/watawuwu/hb-dashboard -h
HTTP Benchmark Tool Dashboard

Usage: app [OPTIONS]

Options:
  -p, --port <PORT>
          Port to listen on [env: HB_DASHBOARD_PORT=]
  -H, --host <HOST>
          Host to listen on [env: HB_DASHBOARD_HOST=] [default: 0.0.0.0]
  -d, --dist-path <DIST_PATH>
          Path to the dist directory [env: HB_DASHBOARD_DIST_PATH=/dist] [default: frontend/dist]
  -c, --tls-cert-path <TLS_CERT_PATH>
          Path to the TLS certificate file [env: HB_DASHBOARD_TLS_CERT_FILE=]
  -k, --tls-key-path <TLS_KEY_PATH>
          Path to the TLS key file [env: HB_DASHBOARD_TLS_KEY_FILE=]
  -u, --datasource-url <DATASOURCE_URL>
          URL to datasource URL(Prometheus) [env: HB_DASHBOARD_DATASOURCE_URL=] [default: http://localhost:9090]
  -h, --help
          Print help
  -V, --version
          Print version
```

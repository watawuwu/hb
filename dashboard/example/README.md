# Example configurations

This directory provides an example of using the [Perses](https://perses.dev) dashboard.
These examples assume the use of Prometheus as the backend storage.

[Perses](https://perses.dev) is first and foremost a dashboard tool that you can use to display a variety of observability data.

## Perses Docker

Change the datasource to use the correct host.

```
# Edit the datasource to use the correct host
$ vi dashboard/example/perses-docker/assets/provisioning/datasource.yaml
```

```
# Start the Perses server
$ docker compose -f dashboard/example/perses-docker/docker-compose.yml up perses
```

## Perses Kubernetes

```
$ vi dashboard/example/perses-kubernetes/assets/provisioning/datasource.yaml
```

```
# Define the name of the release
$ name=hb-perses

# Render the template and apply the resources
$ helm template --dependency-update $name dashboard/example/perses-kubernetes | kubectl apply -f-
```

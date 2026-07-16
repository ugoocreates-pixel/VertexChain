# Edge Rate Limiting Gateway Configuration

This directory contains the Kubernetes configurations to enable edge rate limiting for the VertexChain application, specifically targeting the `POST /gists` endpoint.

## Architecture

To protect the backend from thundering herd problems and coordinated abuse (such as concurrent bursts of requests), we use a layered rate limiting strategy at the edge (Envoy Ingress Gateway):

1. **Local Rate Limit (`envoy.filters.http.local_ratelimit`)**:
   - Enforced directly in each Envoy proxy pod without external network hops.
   - Designed to handle sudden concurrent bursts instantly.
   - Configured to limit `POST /gists` to a burst of 10 requests with a fill rate of 5 tokens per second.

2. **Global Rate Limit (`envoy.filters.http.ratelimit`)**:
   - Enforced cluster-wide using a centralized Redis store and a gRPC Rate Limit service.
   - Ensures coordinated limits across all Envoy proxy replicas.
   - Passes method and path headers as descriptors (`path: /gists`, `method: POST`).

## Components

- **`global-rate-limit-crd.yaml`**: The CustomResourceDefinition (`GlobalRateLimit`) outlining the schema for gateway rate limit configurations.
- **`global-rate-limit-cr.yaml`**: The Custom Resource definition setting the policies specifically for `POST /gists`.
- **`envoy-filter.yaml`**: The EnvoyFilter resource configuring the Envoy proxy to execute local rate limiting and generate global rate-limit descriptors.

## Deployment

Apply the configurations to your Kubernetes cluster:

```bash
# 1. Register the GlobalRateLimit CRD
kubectl apply -f global-rate-limit-crd.yaml

# 2. Deploy the GlobalRateLimit rule
kubectl apply -f global-rate-limit-cr.yaml

# 3. Apply the EnvoyFilter to the gateway
kubectl apply -f envoy-filter.yaml
```

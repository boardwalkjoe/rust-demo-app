# rust-demo-app
A fun little Rust web service that's also practical for testing on OpenShift — with health probes, environment introspection, and a nice landing page.

# Generate lock file and build the image
cargo generate-lockfile
podman build -t rust-demo-app .

# Test locally
podman run -p 8080:8080 rust-demo-app

# Push to OpenShift and deploy
oc new-project rust-demo-app
podman push rust-demo-app <your-registry>/rust-demo-app:latest
# Edit openshift-deploy.yaml to point to your image
oc apply -f openshift-deploy.yaml

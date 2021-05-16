# founder-leaderboard


## database setup

`minikube start`

`helm repo add cockroachdb https://charts.cockroachdb.com/`

`helm repo update`

`helm install cockroachdb cockroachdb/cockroachdb`

`kubectl port-forward svc/cockroachdb-public 26257:26257`

`export DATABASE_URL="postgres://admin:@localhost:26257"`

`sqlx migrate run`

## run sql queries

`kubectl run cockroachdb -it --image=cockroachdb/cockroach:v20.2.9 --rm --restart=Never -- sql --insecure --host=cockroachdb-public`

## db console

`kubectl port-forward service/my-release-cockroachdb-public 8080`

## run the app

`cargo run`

## build the container

`docker build .`
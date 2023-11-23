# How to run

- Run on port 8080

```sh
cargo run
```

- Testing

```sh
### test is the id of sender
curl --location 'http://localhost:8080/wait-for-second-party/test' \
--header 'Content-Type: application/json'
```
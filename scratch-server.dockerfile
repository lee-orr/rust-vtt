FROM scratch

COPY ./docker-contents /app

WORKDIR /app

CMD ["/app/server"]
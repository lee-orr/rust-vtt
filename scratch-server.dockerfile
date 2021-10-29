FROM scratch

COPY ./docker-contents /app
RUN chmod +x /app/server

WORKDIR /app

CMD ["/app/server"]
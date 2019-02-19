class Btsieve {
    constructor(host, port) {
        this.host = host;
        this.port = port;
    }

    url() {
        return "http://" + this.host + ":" + this.port;
    }

    poll_until_matches(chai, query_url) {
        return new Promise((final_res, rej) => {
            chai.request(query_url)
                .get("")
                .end((err, res) => {
                    if (err) {
                        return rej(err);
                    }
                    res.should.have.status(200);
                    if (res.body.matches.length !== 0) {
                        final_res(res.body);
                    } else {
                        setTimeout(() => {
                            this.poll_until_matches(chai, query_url).then(
                                result => {
                                    final_res(result);
                                }
                            );
                        }, 200);
                    }
                });
        });
    }
}

module.exports.create = (host, port) => {
    return new Btsieve(host, port);
};

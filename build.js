var path = require('path');
var safariextz = require('./safariextz');

safariextz(path.resolve('auto/testextension.safariextz'), path.resolve('src/testextension.safariextension'), {
	privateKey: path.resolve('certs/key-openssl098.pem'),
	extensionCer: path.resolve('certs/dev-openssl098.cer'),
	appleDevCer: path.resolve('certs/apple1.cer'),
	appleRootCer: path.resolve('certs/apple2.cer'),
	temp: '/tmp',
});

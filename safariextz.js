//  Safari Extension Packer
//  Copyright 2015 AVAST Software s.r.o.
//  http://www.avast.com
//
//  Packs Safari extensions without interacting with the browser.
//  Works on MacOS, Linux and Windows, as long as you have xar 1.6.1 and openssl installed and in path.
//  Neither xar 1.5 nor 1.7 will work, as they don't support the --sign option

(function () {
    'use strict';

    var execute = require('child_process').exec;
    var when = require('when');
    var fs = require('fs');
    var path = require('path');

    // promisified child_process.exec
    function exec(command, options) {
	console.log('running command', command);
	console.log('in dir', process.cwd());
        var hide_stdout = false;

        if (!options) {
            options = { };
        }
        if (!options.cwd) {
            options.cwd = __dirname;
        }
        if (options.hide_stdout) {
            hide_stdout = true;
            delete options['hide_stdout'];
        }

        //console.log(command);
        return when.promise(function (resolve, reject) {
            execute(command, options, function (err, stdout, stderr) {
                if (!hide_stdout && stdout) {
                    console.log(stdout);
                }
                if (stderr) {
                    console.log(stderr);
                }
                if (err) {
                    reject(err.code);
                }
                else {
                    resolve(stdout);
                }
            })
        });
    }

    // pack and sign Safari extension
    //
    // @param safariextzName name of the pcaked extension
    // @param safariextensionDir source directory
    // @param options {
    //          privateKey   Apple developer private key in PKCS8 format
    //          extensionCer Apple developer certificate in DER encoding
    //          appleDevCer  Apple Worldwide Developer Relations Certification Authority
    //          appleRootCer Apple Root CA
    //          temp [optional] temporary directory, will use cwd if not specified
    // }
    // @return promise
    // 
    function pack(safariextzName, safariextensionDir, options) {
        var temp = options.temp;
        if (!temp) {
            temp = "";
        }

	var leafCertArg = '--cert-loc';
	var intCertArg = '--cert-loc';
	var rootCertArg = '--cert-loc';

        return exec('xar -cf ' + safariextzName + ' ' + ' -C ' + path.dirname(safariextensionDir) + ' ' + path.basename(safariextensionDir))
            .then(function () {
                // find out the signature size by siging anything (in this case the key itself)
                return exec('openssl dgst -sign ' + options.privateKey + ' -binary ' + options.privateKey, { hide_stdout: true, encoding: 'binary' });
            })
            .then(function (signature_buffer) {
                return exec('xar --replace-sign -f ' + safariextzName + ' --data-to-sign ' + path.join(temp, 'digest.dat') + ' --sig-size ' + signature_buffer.length +
                     ' ' + leafCertArg + ' ' + options.extensionCer +
                     ' ' + intCertArg + ' ' + options.appleDevCer +
                     ' ' + rootCertArg + ' ' + options.appleRootCer +
		     ' --sig-offset sigoffset')
            })
	    .then(function () {
		// extract compressed table of contents as toc.dat
		return exec('xartool ' + safariextzName);
	    })
            .then(function () {
		return exec('openssl dgst -sha1 -sign ' + options.privateKey + ' toc.dat', {hide_stdout: true, encoding: 'binary'});
            })
            .then(function (signature) {
		var sigFile = path.join(temp, 'signature.dat');
		fs.writeFileSync(sigFile, signature, {encoding:'binary'});
                return exec('xar --inject-sig ' + sigFile + ' -f ' + safariextzName);
            })
            .then(function () {
                fs.unlink(path.join(temp, 'signature.dat'));
                fs.unlink(path.join(temp, 'digest.dat'));
            });
    }

    module.exports = pack;

})();

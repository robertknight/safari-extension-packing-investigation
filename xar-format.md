# Xar File Format Notes

The _xar_ file format is an archive format used by Safari Extensions amongst other things. See the [original documentation](https://code.google.com/p/xar/) for an overview.

There is also a [more recent fork](https://mackyle.github.io/xar/) which includes support for generating signed xar archives and documentation on how signing works.

## Xar File Structure

See also https://github.com/mackyle/xar/wiki/xarformat

A xar file consists of:
 * A binary header
 * A table of contents (TOC) in XML format, which contains
   the metadata for files in the archive and details of the
   certificates and signature format used to sign the archive,
   if it is signed.
 * The _heap_, which stores signatures and compressed file data.
   The location of file/signature data within the heap is
   specified by (offset, length) data in the TOC.

### Header

The header structure is defined in `include/xar.h`:

```c
struct xar_header {
	uint32_t magic;
	uint16_t size;
	uint16_t version;
	uint64_t toc_length_compressed;
	uint64_t toc_length_uncompressed;
	uint32_t cksum_alg;
};

struct xar_header_ex { /* Only used when cksum_alg == XAR_CKSUM_OTHER */
	uint32_t magic;
	uint16_t size;  /* Must be multiple of 4, but may be any multiple ok so long as Nul terminated toc_cksum_name value is included */
	uint16_t version;
	uint64_t toc_length_compressed;
	uint64_t toc_length_uncompressed;
	uint32_t cksum_alg;
	char     toc_cksum_name[36];	/* Nul terminated and padded toc checksum style attribute value. */
					/* Must not be "none" or empty string "" if cksum_alg == XAR_CKSUM_OTHER */
};
```

### Table of Contents

The TOC is a compressed XML file listing:

 * The files and directories in the archive, including checksums
   used for validation.
 * The details of the signature used to sign the archive. Note
   that the _xar_ tool that ships with OS X does not include
   support for reading or writing signatures. Support for this
   was implemented by Apple in their [xarsig](http://www.opensource.apple.com/source/xar/xar-254/)
   tool.

_offset_ and _length_ tags in the TOC refer to offsets from the start of the _heap_ which immediately follows the end of the TOC.

Below is an example of the TOC from a trivial Safari extension which includes just a "Hello World" toolbar with no logic.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<xar>
 <toc>
  <signature-creation-time>461137009.8</signature-creation-time>
  <checksum style="sha1">
   <size>20</size>
   <offset>0</offset>
  </checksum>
  <creation-time>2015-08-13T05:36:49</creation-time>
  <signature style="RSA">
   <offset>20</offset>
   <size>256</size>
   <KeyInfo xmlns="http://www.w3.org/2000/09/xmldsig#">
    <X509Data>
     <X509Certificate>MIIFZjCCBE6gAwIBAgIIWZCj5GnwFmcwDQYJKoZIhvcNAQEFBQAwgZYxCzAJBgNVBAYTAlVT
MRMwEQYDVQQKDApBcHBsZSBJbmMuMSwwKgYDVQQLDCNBcHBsZSBXb3JsZHdpZGUgRGV2ZWxv
cGVyIFJlbGF0aW9uczFEMEIGA1UEAww7QXBwbGUgV29ybGR3aWRlIERldmVsb3BlciBSZWxh
dGlvbnMgQ2VydGlmaWNhdGlvbiBBdXRob3JpdHkwHhcNMTUwODExMjAzOTE1WhcNMTYwODEw
MjAzOTE1WjBpMRowGAYKCZImiZPyLGQBAQwKV1JDRFZWRDM1MzE+MDwGA1UEAww1U2FmYXJp
IERldmVsb3BlcjogKEE0M0ZXOVJZSzIpIHJvYmVydGtuaWdodEBnbWFpbC5jb20xCzAJBgNV
BAYTAlVTMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAofuFt17KNHWQr59N9sA8
UsEfViZY+oea/7mK9w2a79gAq3WKlJooUjZnSdmjrkCQrCDNc1KHYBOMmlW6xkY+rwOApdvN
TmUyy3LL2EieoTVGeLcIoWrySqRlJPNawml1F48vL4fMvD3OWfZA1jjq8gYZQcsCO0VdoMu7
ulLHhWUDzZlaimb8etdiTcceWoR46ozUngLdtoeeD06tmhmhiFprkMMVfZ+gCq2CD4y8bR8+
mxV3JBkZsOeqjbCLMW1JY0a5VrupFIECrGFkoPtcpPaPWf5ISmCwxmKB9t3ygWXe/93Pp+08
rLWIvwYxovjt8/MyIA/e33sQAScAUO6A3QIDAQABo4IB4jCCAd4wPQYIKwYBBQUHAQEEMTAv
MC0GCCsGAQUFBzABhiFodHRwOi8vb2NzcC5hcHBsZS5jb20vb2NzcC13d2RyMDIwHQYDVR0O
BBYEFEHHDudueeXpiG4RwpW/kdwBZp6nMAwGA1UdEwEB/wQCMAAwHwYDVR0jBBgwFoAUiCcX
Cam2GGCL7Ou69kdZxVJUo7cwggEPBgNVHSAEggEGMIIBAjCB/wYJKoZIhvdjZAUBMIHxMIHD
BggrBgEFBQcCAjCBtgyBs1JlbGlhbmNlIG9uIHRoaXMgY2VydGlmaWNhdGUgYnkgYW55IHBh
cnR5IGFzc3VtZXMgYWNjZXB0YW5jZSBvZiB0aGUgdGhlbiBhcHBsaWNhYmxlIHN0YW5kYXJk
IHRlcm1zIGFuZCBjb25kaXRpb25zIG9mIHVzZSwgY2VydGlmaWNhdGUgcG9saWN5IGFuZCBj
ZXJ0aWZpY2F0aW9uIHByYWN0aWNlIHN0YXRlbWVudHMuMCkGCCsGAQUFBwIBFh1odHRwOi8v
d3d3LmFwcGxlLmNvbS9hcHBsZWNhLzAOBgNVHQ8BAf8EBAMCB4AwFwYDVR0lAQH/BA0wCwYJ
KoZIhvdjZAQIMBMGCiqGSIb3Y2QGAQUBAf8EAgUAMA0GCSqGSIb3DQEBBQUAA4IBAQBtXZ15
DvZaVN+8Ia5pjtUzGVksP6Lg3QDLwQDCcQxJufWa/iYwKxA+EaVfP76IdkKHijeiOoWmvn37
7qjNNfQ6LEsZl1CP3/iVTpKKFcR8T/iVGFNkD5+fHYM7Ntfszt4oDG+viBPrGp4SqmrrILKe
7zHY4C7imqGbt3sVx6xWGorr5JfqqMJU/mS3uGluUysII0MNPbLiI6bk8GdYiYNYTdiYElQV
ru6LftES+ozCNyoOdUvTY3b0kSmCOzpX6c8iaQU8IsHfSu1b8nLvWWsLECuD3kuFi7v9Tyan
Rr+2bCXWUQZ4QjFOHghCRCTxm6GeQou5StW9BQ4+cM0dT4Ll</X509Certificate>
     <X509Certificate>MIIEIzCCAwugAwIBAgIBGTANBgkqhkiG9w0BAQUFADBiMQswCQYDVQQGEwJVUzETMBEGA1UE
ChMKQXBwbGUgSW5jLjEmMCQGA1UECxMdQXBwbGUgQ2VydGlmaWNhdGlvbiBBdXRob3JpdHkx
FjAUBgNVBAMTDUFwcGxlIFJvb3QgQ0EwHhcNMDgwMjE0MTg1NjM1WhcNMTYwMjE0MTg1NjM1
WjCBljELMAkGA1UEBhMCVVMxEzARBgNVBAoMCkFwcGxlIEluYy4xLDAqBgNVBAsMI0FwcGxl
IFdvcmxkd2lkZSBEZXZlbG9wZXIgUmVsYXRpb25zMUQwQgYDVQQDDDtBcHBsZSBXb3JsZHdp
ZGUgRGV2ZWxvcGVyIFJlbGF0aW9ucyBDZXJ0aWZpY2F0aW9uIEF1dGhvcml0eTCCASIwDQYJ
KoZIhvcNAQEBBQADggEPADCCAQoCggEBAMo4VKbLVqrIJDlI6Yzu7F+4fyaRvDRTes58Y4Bh
d2RepQcjtjn+UC0VVlhwLX7EbsFKhT4v8N6EGqFXya97GP9q+hUSSRUIGayq2yoy7ZZjaFIV
PYyK7L9rGJXgA6wBfZcFZ84OhZU3au0Jtq5nzVFkn8Zc0bxXbmc1gHY2pIeBbjiP2CsVTnsl
2Fq/ToPBjdKT1RpxtWCcnTNOVfkSWAyGuBYNweV3RY1QSLorLeSUheHoxJ3GaKWwo/xnfnC6
AllLd0KRObn1zeFM78A7SIym5SFd/Wpqu6cWNWDS5q3zRinJ6MOL6XnAamFnFbLw/eVovGJf
bs+Z3e8bY/6SZasCAwEAAaOBrjCBqzAOBgNVHQ8BAf8EBAMCAYYwDwYDVR0TAQH/BAUwAwEB
/zAdBgNVHQ4EFgQUiCcXCam2GGCL7Ou69kdZxVJUo7cwHwYDVR0jBBgwFoAUK9BpR5R2Cf70
a40uQKb3R01/CF4wNgYDVR0fBC8wLTAroCmgJ4YlaHR0cDovL3d3dy5hcHBsZS5jb20vYXBw
bGVjYS9yb290LmNybDAQBgoqhkiG92NkBgIBBAIFADANBgkqhkiG9w0BAQUFAAOCAQEA2jIA
lsVUlNM7gjdmfS5o1cPGuMsmjEiQzxMkakaOY9Tw0BMG3djEwTcV8jMTOSYtzi5VQOMLA6/6
EsLnDSG41YDPrCgvzi2zTq+GGQTG6VDdTClHECP8bLsbmGtIieFbnd5G2zWFNe8+0OJYSzj0
7XVaH1xwHVY5EuXhDRHkiSUGvdW0FY5e0FmXkOlLgeLfGK9EdB4ZoDpHzJEdOusjWv6lLZf3
e7vWh0ZChetSPSayY6i0scqP9Mzis8hH4L+aWYP62phTKoL1fGUuldkzXfXtZcwxN8VaBOhr
4eeIA0p1npsoy0pAiGVDdd3LOiUjxZ5X+C7O0qmSXnMuLyV1FQ==</X509Certificate>
     <X509Certificate>MIIEuzCCA6OgAwIBAgIBAjANBgkqhkiG9w0BAQUFADBiMQswCQYDVQQGEwJVUzETMBEGA1UE
ChMKQXBwbGUgSW5jLjEmMCQGA1UECxMdQXBwbGUgQ2VydGlmaWNhdGlvbiBBdXRob3JpdHkx
FjAUBgNVBAMTDUFwcGxlIFJvb3QgQ0EwHhcNMDYwNDI1MjE0MDM2WhcNMzUwMjA5MjE0MDM2
WjBiMQswCQYDVQQGEwJVUzETMBEGA1UEChMKQXBwbGUgSW5jLjEmMCQGA1UECxMdQXBwbGUg
Q2VydGlmaWNhdGlvbiBBdXRob3JpdHkxFjAUBgNVBAMTDUFwcGxlIFJvb3QgQ0EwggEiMA0G
CSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDkkakJH5HbHkdQ6wXtXnmELes2oldMVeyLGYne
+Uts9QerIjAC6Bg++FAJ039BqJj50cpmnCRrEdCju+QbKsMflZ56DKRHi1vUFjczy8QPTc4U
adHJGXL1XQ7Vf1+b8iUDulWPTV0N8WQ1IxVLFVkds5T39pyez1C6wVhQZ48ItCD3y6wsIG9w
tj8BMIy3Q88PnT3zK0koGsj+zrW5DtleHNbLPbU6rfQPDgCSC7EhFi501TwN22IWq6NxkkdT
VcGvL0Gz+PvjcM3mo0xFfh9Ma1CWQYnEdGILEINBhzOKgbEwWOxaBDKMaLOPHd5lc/9nXmW8
Sdh2nzMUZaF3lMktAgMBAAGjggF6MIIBdjAOBgNVHQ8BAf8EBAMCAQYwDwYDVR0TAQH/BAUw
AwEB/zAdBgNVHQ4EFgQUK9BpR5R2Cf70a40uQKb3R01/CF4wHwYDVR0jBBgwFoAUK9BpR5R2
Cf70a40uQKb3R01/CF4wggERBgNVHSAEggEIMIIBBDCCAQAGCSqGSIb3Y2QFATCB8jAqBggr
BgEFBQcCARYeaHR0cHM6Ly93d3cuYXBwbGUuY29tL2FwcGxlY2EvMIHDBggrBgEFBQcCAjCB
thqBs1JlbGlhbmNlIG9uIHRoaXMgY2VydGlmaWNhdGUgYnkgYW55IHBhcnR5IGFzc3VtZXMg
YWNjZXB0YW5jZSBvZiB0aGUgdGhlbiBhcHBsaWNhYmxlIHN0YW5kYXJkIHRlcm1zIGFuZCBj
b25kaXRpb25zIG9mIHVzZSwgY2VydGlmaWNhdGUgcG9saWN5IGFuZCBjZXJ0aWZpY2F0aW9u
IHByYWN0aWNlIHN0YXRlbWVudHMuMA0GCSqGSIb3DQEBBQUAA4IBAQBcNplMLXi37Yyb3PN3
m/J20ncwT8EfhYOFG5k9RzfyqZtAjizUsZAS2L70c5vu0mQPy3lPNNiiPvl4/2vIB+x9OYOL
UyDTOMSxv5pPCmv/K/xZpwUJfBdAVhEedNO3iyM7R6PVbyTi69G3cN8PReEnyvFteO3ntRcX
qNx+IjXKJdXZD9Zr1KIkIxH3oayPc4FgxhtbCS+SsvhESPBgOJ4V9T0mZyCKM2r3DYLP3uuj
L/lTaltkwGMzd/c6ByxW69oPIQ7aunMZT7XZNn/Bh1XZp5m5MkL72NVxnn6hUrcbvZNCJBIq
xw8dtk2cXmPIS4AXUKqK1drk/NAJBzewdXUh</X509Certificate>
    </X509Data>
   </KeyInfo>
  </signature>
  <file id="1">
   <name>testextension.safariextension</name>
   <type>directory</type>
   <inode>7539443</inode>
   <deviceno>16777218</deviceno>
   <mode>0755</mode>
   <uid>502</uid>
   <user>robert</user>
   <gid>20</gid>
   <group>staff</group>
   <atime>2015-08-13T05:36:37Z</atime>
   <mtime>2015-08-12T19:02:55Z</mtime>
   <ctime>2015-08-12T19:12:53Z</ctime>
   <FinderCreateTime>
    <time>1970-01-01T00:00:00</time>
    <nanoseconds>140359531233280</nanoseconds>
   </FinderCreateTime>
   <ea id="0">
    <name>com.apple.quarantine</name>
    <archived-checksum style="sha1">a1b894ae0129ec008e196adfce577c262929f379</archived-checksum>
    <extracted-checksum style="sha1">bf2e12c7ec0f17e0b388f7464b125395610b0af7</extracted-checksum>
    <encoding style="application/x-gzip"/>
    <size>21</size>
    <offset>276</offset>
    <length>23</length>
   </ea>
   <file id="2">
    <name>ext.html</name>
    <type>file</type>
    <data>
     <archived-checksum style="sha1">ce3fa8c06ffe4490932afe16f4865eceff2ac2dc</archived-checksum>
     <extracted-checksum style="sha1">8fdba5761aebde98d6081ab6aa408e53703902b3</extracted-checksum>
     <encoding style="application/x-gzip"/>
     <size>118</size>
     <offset>299</offset>
     <length>79</length>
    </data>
   </file>
   <file id="3">
    <name>Info.plist</name>
    <type>file</type>
    <data>
     <archived-checksum style="sha1">f98207e7ad26c52dd3f30adcff579e5df58ee8b0</archived-checksum>
     <extracted-checksum style="sha1">d00e90b7fef19165c230ee71458afb64531e9fb6</extracted-checksum>
     <encoding style="application/x-gzip"/>
     <size>1048</size>
     <offset>378</offset>
     <length>418</length>
    </data>
   </file>
  </file>
 </toc>
</xar>
```

### Heap

The heap contains the actual data referenced by the TOC.

_Extensions generated by Safari (tested with Safari 9.0) include additional data beyond the end of the heap. This data does not appear to be used when installing the extension or verifying the signature of the archive._

## Signed Xar Archives

For Safari extensions, signatures were added to the Xar file format.

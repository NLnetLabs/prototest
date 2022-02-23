//! Creates a self-signed RPKI CA certificate and writes it to stdout.

use chrono::{Duration, Utc};
use rand::rngs::OsRng;
use rsa::{PaddingScheme, RsaPrivateKey, RsaPublicKey};
use rsa::pkcs1::ToRsaPublicKey;
use sha1::{Digest, Sha1};
use sha2::Sha256;
use testnetproto::recipe::{core, der};

fn main() {
    // Step 1.  Generate a key pair.
    let mut rng = OsRng;
    let private_key = RsaPrivateKey::new(
        &mut rng, 2048
    ).expect("failed to generate a key");
    let public_key = RsaPublicKey::from(&private_key);
    let key_id = Sha1::digest(public_key.to_pkcs1_der().unwrap().as_der());
    let key_id = key_id.as_slice();

    // Step 2. Generate a recipe for the certificate content.
    let tbs = der::sequence([
        // version
        der::explicit(0, der::integer(2)),

        // serialNumber
        der::integer(12),

        // signature
        der::sequence([
            der::oid([1, 2, 840, 113549, 1, 1, 11]),
            der::null(),
        ]),

        // issuer
        der::sequence([
            der::set([
                der::sequence([
                    der::oid([2, 5, 4, 3]),
                    der::printable_string("deadbeef")
                ]),
            ])
        ]),

        // validity
        der::sequence([
            der::x509_time(Utc::now()),
            der::x509_time(Utc::now() + Duration::days(365)),
        ]),

        // subject
        der::sequence([
            der::set([
                der::sequence([
                    der::oid([2, 5, 4, 3]),
                    der::printable_string("deadbeef")
                ]),
            ])
        ]),

        // subjectPublicKeyInfo
        der::sequence([
            // algorithm
            der::sequence([
                der::oid([1, 2, 840, 113549, 1, 1, 1]),
                der::null(),
            ]),

            // subjectPublicKey
            der::bitstring(
                0,
                Vec::from(public_key.to_pkcs1_der().unwrap().as_der()).into()
            )
        ]),

        // extensions
        der::explicit(3, der::sequence([
            // BasicConstraints
            der::sequence([
                der::oid([2, 5, 29, 19]),
                der::boolean(true),
                der::octetstring(
                    der::sequence([
                        // cA
                        der::boolean(true),
                    ])
                )
            ]),

            // SubjectKeyIdentifier
            der::sequence([
                der::oid([2, 5, 29, 14]),
                der::octetstring(
                    der::octetstring(
                        Vec::from(key_id.as_ref()).into()
                    )
                )
            ]),

            // AuthorityKeyIdentifier
            der::sequence([
                der::oid([2, 5, 29, 35]),
                der::octetstring(
                    der::sequence([
                        der::value(der::context(false, 0),
                            Vec::from(key_id.as_ref()).into()
                        )
                    ])
                )
            ]),

            // KeyUsage
            der::sequence([
                der::oid([2, 5, 29, 15]),
                der::boolean(true),
                der::octetstring(
                    der::bitstring(1, [0b0000_0110].into())
                )
            ]),

            // SubjectInfoAccess
            der::sequence([
                der::oid([1, 3, 6, 1, 5, 5, 7, 1, 11]),
                der::octetstring(
                    der::sequence([
                        // caRepository
                        der::sequence([
                            der::oid([1, 3, 6, 1, 5, 5, 7, 48, 5]),
                            der::value(
                                der::context(false, 6),
                                "rsync://rpki.example.com/ta/".into()
                            )
                        ]),

                        // rpkiManifest
                        der::sequence([
                            der::oid([1, 3, 6, 1, 5, 5, 7, 48, 10]),
                            der::value(
                                der::context(false, 6),
                                "rsync://rpki.example.com/ta/ta.mft".into()
                            )
                        ]),
                    ])
                )
            ]),

            // CertificatePolicies
            der::sequence([
                der::oid([2, 5, 29, 32]),
                der::boolean(true),
                der::octetstring(
                    der::sequence([
                        der::sequence([
                            der::oid([1, 3, 6, 1, 5, 5, 7, 14, 2])
                        ])
                    ])
                )
            ]),

            // IP Resources
            der::sequence([
                der::oid([1, 3, 6, 1, 5, 5, 7, 1, 7]),
                der::boolean(true),
                der::octetstring(
                    der::sequence([ // IPAddrBlocks
                        der::sequence([ // IPAddressFamily
                            der::octetstring([0, 1].into()),   // v4
                            der::sequence([
                                der::bitstring(0, b"".into()),
                            ]),
                        ]),
                        der::sequence([ // IPAddressFamily
                            der::octetstring([0, 2].into()),   // v6
                            der::sequence([
                                der::bitstring(0, b"".into()),
                            ]),
                        ])
                    ]),
                )
            ]),

            // AS Resources
            der::sequence([
                der::oid([1, 3, 6, 1, 5, 5, 7, 1, 8]),
                der::boolean(true),
                der::octetstring(
                    der::sequence([ // AsIdentifiers
                        der::explicit(0,
                            der::sequence([ // asIdsOrRanges
                                der::sequence([
                                    der::integer(0),
                                    der::integer([
                                        0, 0xFF, 0xFF, 0xFF, 0xFF
                                    ])
                                ].into())
                            ])
                        ),
                    ])
                )
            ]),
        ])),
    ]);

    // Step 3.  Sign the certificate content.
    let signature = private_key.sign(
        PaddingScheme::new_pkcs1v15_sign(
            Some(rsa::hash::Hash::SHA2_256)
        ),
        Sha256::digest(&tbs.to_vec()).as_slice(),
    ).unwrap();

    // Step 4. Create a recipe for the full certificate.
    let cert = der::sequence([
        tbs,
        der::sequence([
            der::oid([1, 2, 840, 113549, 1, 1, 11]),
            der::null(),
        ]),
        der::bitstring(0, signature.into()),
    ]);

    // Step 5. Write the certificate to stdout.
    let stdout = std::io::stdout();
    core::write_recipe(
        &cert,
        &mut stdout.lock(),
    ).unwrap();
}


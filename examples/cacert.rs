//! Creates an RPKI CA certificate and writes it to stdout.
//!
//! XXX So far, we are only creating the TBSCertificate bit.

use std::io::Write;
use base64::write::EncoderWriter;
use testnetproto::recipe::{core, der};

fn main() {
    let tbs = der::sequence([
        // version
        der::context_integer(0, 2),

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

        // XXX ...
    ]);

    let mut stdout = std::io::stdout();
    core::write_recipe(
        &tbs,
        &mut EncoderWriter::new(stdout.lock(), base64::STANDARD)
    ).unwrap();
    writeln!(stdout, "").unwrap();
}


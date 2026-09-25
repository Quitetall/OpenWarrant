# Signing must not make evidence stale

Under OW-WAR-0133 a receipt from a gate with no declared inputs binds the whole tree, and authorization/response/attestation records are not excluded — so a human signature recorded after evidence stales every receipt, forcing two separate human sittings (authorize, then resolve). Decide whether authority records belong in the source Exclusions, or whether gates should declare inputs, and make one sitting possible.

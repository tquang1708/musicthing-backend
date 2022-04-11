read -n1 -p "Run initialization steps? (yY/nN)" option
echo

case $option in
    y|Y)
        echo "Dropping old musicthing-metadb database. Answer 'y' to confirm."
        dropdb -i musicthing-metadb
        echo "Creating musicthing-metadb database."
        createdb musicthing-metadb
        echo "Setting up tables."
        psql -d musicthing-metadb -f musicthing_metadb_init.sql

        echo "Creating default music directory at ../music"
        echo "The backend will pick up music files stored in here."
        echo "Currently only supports .mp3/.flac"
        read -n1 -p "Proceed? (yY/nN)" default_music_dir
        echo
        case $default_music_dir in
        y|Y)
            echo "Creating music directory at ../music if doesn't already exist."
            mkdir -p "../music"
            ;;
        *)
            echo "Skipping creation of music directory."
            echo "Remember to point to your desired music directory in"
            echo "config.json before starting up the backend."
            ;;
        esac

        echo "Creating default art directory at ./art"
        echo "Arts used for display on frontend will be stored here."
        mkdir -p "./art"

        echo "Generating cert.pem and key.pem for TLS protocol for HTTPS"
        read -n1 -p "Proceed? (yY/nN)" generate_pem
        echo
        case $generate_pem in
        y|Y)
            echo "Creating self_signed_certs directory at ./self_signed_certs"
            mkdir "self-signed-certs"
            echo "Generating .pem files with 4096-bit RSA for key (unencrypted with DES), self-signed cert valid for 365 days."
            # taken from the arch wiki on OpenSSL
            # feel free to adjust the parameters here if you would rather use different settings for generation
            # https://wiki.archlinux.org/title/OpenSSL#Generate_a_self-signed_certificate_with_private_key_in_a_single_command
            openssl req -x509 -newkey rsa:4096 -days 365 -nodes -keyout "self-signed-certs/key.pem" -out "self-signed-certs/cert.pem"
            ;;
        *)
            echo "Skipping generation of .pem files"
            echo "Remember to change the 'use_tls' option to 'false' in"
            echo "config.json if you are planning to communicate over HTTP"
            ;;
        esac
        ;;
    n|N) continue ;;
    *) 
        echo "Unrecognized input"
        exit 1
        ;;
esac

echo "Starting up backend."
cargo run

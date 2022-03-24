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

        echo "Creating default music directory at ../music-directory."
        read -n1 -p "Proceed? (yY/nN)" default_music_dir
        echo

        case $default_music_dir in
        y|Y)
            echo "Creating music directory at ../music-directory if doesn't already exist."
            echo "Move your music files here for the backend to pick up."
            echo "Currently only supports .mp3/.flac"
            mkdir -p "../music-directory"
            ;;
        *)
            echo "Skipping creation of music directory."
            echo "Remember to point to your desired music directory in "
            echo "config.json before starting up the backend."
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

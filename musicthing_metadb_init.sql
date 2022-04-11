--
-- PostgreSQL database dump
--

-- Dumped from database version 13.6
-- Dumped by pg_dump version 13.6

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: album; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.album (
    album_id integer NOT NULL,
    album_name text
);


--
-- Name: album_album_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.album_album_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: album_album_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.album_album_id_seq OWNED BY public.album.album_id;


--
-- Name: album_art; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.album_art (
    album_id integer NOT NULL,
    art_id integer NOT NULL
);


--
-- Name: album_track; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.album_track (
    album_id integer NOT NULL,
    track_id integer NOT NULL,
    track_no integer,
    disc_no integer
);


--
-- Name: art; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.art (
    art_id integer NOT NULL,
    hash bytea NOT NULL,
    path text NOT NULL
);


--
-- Name: art_art_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.art_art_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: art_art_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.art_art_id_seq OWNED BY public.art.art_id;


--
-- Name: artist; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.artist (
    artist_id integer NOT NULL,
    artist_name text
);


--
-- Name: artist_album; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.artist_album (
    artist_id integer,
    album_id integer
);


--
-- Name: artist_art; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.artist_art (
    artist_id integer NOT NULL,
    art_id integer NOT NULL
);


--
-- Name: artist_artist_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.artist_artist_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: artist_artist_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.artist_artist_id_seq OWNED BY public.artist.artist_id;


--
-- Name: artist_track; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.artist_track (
    artist_id integer NOT NULL,
    track_id integer NOT NULL
);


--
-- Name: track; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.track (
    track_id integer NOT NULL,
    track_name text,
    path text NOT NULL,
    last_modified timestamp without time zone NOT NULL,
    length_seconds integer NOT NULL
);


--
-- Name: track_art; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.track_art (
    track_id integer NOT NULL,
    art_id integer NOT NULL
);


--
-- Name: track_track_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.track_track_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: track_track_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.track_track_id_seq OWNED BY public.track.track_id;


--
-- Name: album album_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.album ALTER COLUMN album_id SET DEFAULT nextval('public.album_album_id_seq'::regclass);


--
-- Name: art art_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.art ALTER COLUMN art_id SET DEFAULT nextval('public.art_art_id_seq'::regclass);


--
-- Name: artist artist_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist ALTER COLUMN artist_id SET DEFAULT nextval('public.artist_artist_id_seq'::regclass);


--
-- Name: track track_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.track ALTER COLUMN track_id SET DEFAULT nextval('public.track_track_id_seq'::regclass);


--
-- Name: album album_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.album
    ADD CONSTRAINT album_pkey PRIMARY KEY (album_id);


--
-- Name: art art_hash_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.art
    ADD CONSTRAINT art_hash_key UNIQUE (hash);


--
-- Name: art art_path_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.art
    ADD CONSTRAINT art_path_key UNIQUE (path);


--
-- Name: art art_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.art
    ADD CONSTRAINT art_pkey PRIMARY KEY (art_id);


--
-- Name: artist artist_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist
    ADD CONSTRAINT artist_pkey PRIMARY KEY (artist_id);


--
-- Name: track track_path_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.track
    ADD CONSTRAINT track_path_key UNIQUE (path);


--
-- Name: track track_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.track
    ADD CONSTRAINT track_pkey PRIMARY KEY (track_id);


--
-- Name: album_art unique_album_id_art; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.album_art
    ADD CONSTRAINT unique_album_id_art UNIQUE (album_id);


--
-- Name: artist_album unique_album_id_artist; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist_album
    ADD CONSTRAINT unique_album_id_artist UNIQUE (album_id);


--
-- Name: artist_art unique_artist_id_art; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist_art
    ADD CONSTRAINT unique_artist_id_art UNIQUE (artist_id);


--
-- Name: artist unique_artist_name; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist
    ADD CONSTRAINT unique_artist_name UNIQUE (artist_name);


--
-- Name: artist_track unique_track_id; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist_track
    ADD CONSTRAINT unique_track_id UNIQUE (track_id);


--
-- Name: album_track unique_track_id_album; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.album_track
    ADD CONSTRAINT unique_track_id_album UNIQUE (track_id);


--
-- Name: track_art unique_track_id_art; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.track_art
    ADD CONSTRAINT unique_track_id_art UNIQUE (track_id);


--
-- Name: album_art album_art_album_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.album_art
    ADD CONSTRAINT album_art_album_id_fkey FOREIGN KEY (album_id) REFERENCES public.album(album_id) ON DELETE CASCADE;


--
-- Name: album_art album_art_art_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.album_art
    ADD CONSTRAINT album_art_art_id_fkey FOREIGN KEY (art_id) REFERENCES public.art(art_id);


--
-- Name: album_track album_track_album_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.album_track
    ADD CONSTRAINT album_track_album_id_fkey FOREIGN KEY (album_id) REFERENCES public.album(album_id) ON DELETE CASCADE;


--
-- Name: album_track album_track_track_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.album_track
    ADD CONSTRAINT album_track_track_id_fkey FOREIGN KEY (track_id) REFERENCES public.track(track_id) ON DELETE CASCADE;


--
-- Name: artist_album artist_album_album_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist_album
    ADD CONSTRAINT artist_album_album_id_fkey FOREIGN KEY (album_id) REFERENCES public.album(album_id) ON DELETE CASCADE;


--
-- Name: artist_album artist_album_artist_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist_album
    ADD CONSTRAINT artist_album_artist_id_fkey FOREIGN KEY (artist_id) REFERENCES public.artist(artist_id) ON DELETE CASCADE;


--
-- Name: artist_art artist_art_art_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist_art
    ADD CONSTRAINT artist_art_art_id_fkey FOREIGN KEY (art_id) REFERENCES public.art(art_id);


--
-- Name: artist_art artist_art_artist_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist_art
    ADD CONSTRAINT artist_art_artist_id_fkey FOREIGN KEY (artist_id) REFERENCES public.artist(artist_id) ON DELETE CASCADE;


--
-- Name: artist_track artist_track_artist_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist_track
    ADD CONSTRAINT artist_track_artist_id_fkey FOREIGN KEY (artist_id) REFERENCES public.artist(artist_id) ON DELETE CASCADE;


--
-- Name: artist_track artist_track_track_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist_track
    ADD CONSTRAINT artist_track_track_id_fkey FOREIGN KEY (track_id) REFERENCES public.track(track_id) ON DELETE CASCADE;


--
-- Name: track_art track_art_art_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.track_art
    ADD CONSTRAINT track_art_art_id_fkey FOREIGN KEY (art_id) REFERENCES public.art(art_id);


--
-- Name: track_art track_art_track_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.track_art
    ADD CONSTRAINT track_art_track_id_fkey FOREIGN KEY (track_id) REFERENCES public.track(track_id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--


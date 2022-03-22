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
-- Name: album_track; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.album_track (
    album_id integer NOT NULL,
    track_id integer NOT NULL,
    track_no integer,
    disc_no integer
);


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
    last_modified timestamp without time zone NOT NULL
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
-- Name: artist_album unique_album_id_artist; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.artist_album
    ADD CONSTRAINT unique_album_id_artist UNIQUE (album_id);


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
-- PostgreSQL database dump complete
--


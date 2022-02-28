function Button(props) {
    console.log(props)
    const { buttonName } = props;
    return (
        <button>
            {buttonName}
        </button>
    );
}

ReactDOM.render(
    <React.StrictMode>
        <Button buttonName= "button" />
    </React.StrictMode>, 
    document.getElementById('button_container')
);
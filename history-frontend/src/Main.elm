module Main exposing (Model, Msg(..), update, view)

import Array exposing (Array)
import Browser exposing (Document)
import Element exposing (Element, alignRight, centerX, centerY, column, el, fill, height, htmlAttribute, image, padding, px, rgb255, row, shrink, spacing, text, width)
import Element.Background as Background
import Element.Border as Border
import Element.Font as Font
import Element.Input exposing (slider)
import Html exposing (Html)
import Html.Attributes exposing (class, style)
import Http
import Json.Decode exposing (Decoder)
import Time


main =
    Browser.document { init = init, update = update, view = documentView, subscriptions = subscriptions }



-- MODEL


type RemoteData a
    = Failure
    | Loading
    | Success a


remoteDataToMaybe : RemoteData a -> Maybe a
remoteDataToMaybe data =
    case data of
        Success inner ->
            Just inner

        _ ->
            Nothing


type alias Model =
    { currentTime : String
    , history : RemoteData History
    , currentIndex : Int
    , playing : Bool
    }


type alias LogoState =
    { time : String
    , logo : String
    }


type alias History =
    Array LogoState


getHistory : Cmd Msg
getHistory =
    Http.get { url = "https://logo-png.app.iterate.no/api/v1/history", expect = Http.expectJson GotHistory historyDecoder }


historyDecoder : Decoder History
historyDecoder =
    Json.Decode.array logoStateDecoder


logoStateDecoder : Decoder LogoState
logoStateDecoder =
    Json.Decode.map2 LogoState
        (Json.Decode.field "time" Json.Decode.string)
        (Json.Decode.field "logo" Json.Decode.string)


init : () -> ( Model, Cmd Msg )
init _ =
    ( { currentTime = ""
      , currentIndex = 0
      , history = Loading
      , playing = False
      }
    , getHistory
    )



-- UPDATE


type Msg
    = SetCurrentTime String
    | GotHistory (Result Http.Error History)
    | SetSlider Float
    | TogglePlaying
    | GoToNextState Time.Posix


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        SetCurrentTime newCurrentTime ->
            ( { model | currentTime = newCurrentTime }, Cmd.none )

        GotHistory result ->
            case result of
                Ok history ->
                    ( { model | history = Success history }, Cmd.none )

                Err _ ->
                    ( { model | history = Failure }, Cmd.none )

        SetSlider newValue ->
            ( { model | currentIndex = round newValue }, Cmd.none )

        TogglePlaying ->
            ( { model | playing = not model.playing }, Cmd.none )

        GoToNextState _ ->
            let
                historyLength =
                    model.history |> remoteDataToMaybe |> Maybe.map Array.length |> Maybe.withDefault 0

                newIndex =
                    if model.currentIndex >= historyLength - 1 then
                        0

                    else
                        model.currentIndex + 1
            in
            ( { model | currentIndex = newIndex }, Cmd.none )



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions model =
    if model.playing then
        Time.every 20 GoToNextState

    else
        Sub.none



-- VIEW


documentView : Model -> Document Msg
documentView model =
    { title = "Logo History", body = [ view model ] }


view : Model -> Html Msg
view model =
    Element.layout [] (logoWithControls model)


logoWithControls : Model -> Element Msg
logoWithControls model =
    column [ width shrink, centerY, centerX, spacing 30 ]
        [ logoState model, controls model ]


emptyPng : String
emptyPng =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkqAcAAIUAgUW0RjgAAAAASUVORK5CYII"


logoState : Model -> Element Msg
logoState model =
    case model.history of
        Loading ->
            text "Loading..."

        Failure ->
            text "Error!"

        Success history ->
            let
                imageData =
                    Array.get model.currentIndex history |> Maybe.map .logo |> Maybe.withDefault emptyPng

                src =
                    "data:image/png;base64," ++ imageData
            in
            image
                [ htmlAttribute (class "pixelated")
                , width (px 912)
                ]
                { src = src, description = "logo" }


controls : Model -> Element Msg
controls model =
    row
        [ width fill ]
        [ runButton model, timeSlider model ]


timeSlider : Model -> Element Msg
timeSlider model =
    slider
        [ height (px 30)
        , Element.behindContent
            (el
                [ width fill
                , height (px 2)
                , centerY
                , Background.color (rgb255 128 128 128)
                , Border.rounded 2
                ]
                Element.none
            )
        ]
        { onChange = SetSlider
        , label = Element.Input.labelAbove [] (text "Timeline")
        , min = 0
        , max = model.history |> remoteDataToMaybe |> Maybe.map (\a -> Array.length a - 1) |> Maybe.withDefault 0 |> toFloat
        , value = model.currentIndex |> toFloat
        , step = Just 1
        , thumb = Element.Input.defaultThumb
        }


runButton : Model -> Element Msg
runButton model =
    Element.Input.button [ Border.color (rgb255 128 128 128), Border.rounded 2 ]
        { onPress = Just TogglePlaying
        , label =
            text <|
                if model.playing then
                    "pause"

                else
                    "play"
        }

# Facial rendering source scan: parthubhe/Agentic_MetaHumans

Repository: https://github.com/parthubhe/Agentic_MetaHumans

Pinned source revision: 43434f137dd68e6b0e41c13b7ed34e25b4e9db04

Discovery topic: lip sync 3D

Description: Python framework for 3D agentic AI avatars (MetaHuman/UE). Features real-time lip-sync (Audio2Face), custom text-based emotion driving expressions, and multiple LangChain agent examples (Cafe, PA, Nurse). Along with sample HTML,CSS,JS website.

Reported license: Apache-2.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["Music/close_eyes.mp3", "Music/ellie_goulding___my_eyes.mp3", "Music/ellie_goulding_my_eyes.mp3", "Music/ellie_goulding_my_eyes_official_song.mp3", "Music/my_eyes.mp3", "Music/my_eyes_by_ellie_goulding.mp3", "Music/my_eyes_by_goulding_ellie.mp3", "Music/my_eyes_ellie_goulding.mp3", "Music/my_eyes_song.mp3"], "lip": ["Music/born_slippy_by_underworld.mp3"]}

## Source file: db_main.py

{
  "path": "db_main.py",
  "blob_sha": "7dd13cbee3da0db2106c6b990e2b4ffaa1c3a047",
  "sha256": "6f501b6a4b2088128942ffbcdb228d65c448be366db980ee79410cbeffb6a5b5",
  "lines": 283,
  "bytes": 12699,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 12
  },
  "url": "https://github.com/parthubhe/Agentic_MetaHumans/blob/43434f137dd68e6b0e41c13b7ed34e25b4e9db04/db_main.py",
  "function_signature_candidates": [
    {
      "line": 45,
      "text": "def home():"
    },
    {
      "line": 49,
      "text": "def agents():"
    },
    {
      "line": 53,
      "text": "def index():"
    },
    {
      "line": 59,
      "text": "def personal_assistant():"
    },
    {
      "line": 65,
      "text": "def agentic_nurse():"
    },
    {
      "line": 74,
      "text": "def welcome():"
    },
    {
      "line": 91,
      "text": "def chat():"
    },
    {
      "line": 156,
      "text": "def chef_dashboard():"
    },
    {
      "line": 170,
      "text": "def welcome_pa():"
    },
    {
      "line": 186,
      "text": "def chat_pa():"
    },
    {
      "line": 217,
      "text": "def welcome_na():"
    },
    {
      "line": 233,
      "text": "def chat_na():"
    }
  ]
}

```text


import os
import re
from flask import Flask, request, jsonify, render_template
from Utils import a2f_utils
from Utils.a2f_utils import load_usd_file, activate_stream_livelink, set_stream_livelink_settings, set_audio_looping, send_audio_to_audio2face, set_audio2face_root_path # Added set_audio2face_root_path import
from CRA_Utils import database
import sqlite3
from cafe_receptionist import show_menu, create_cafe_kiosk_agent, process_response_text
from personal_assistant import create_pa_agent
from nurse_agent import create_nurse_agent, process_response_text_nurse
from Utils.audio_processing_utils import process_response_text_generic
from Utils.emoClassier import predict_emotion, generate_emotion_payload_from_probabilities
from dotenv import load_dotenv

load_dotenv()
os.environ["GROQ_API_KEY"] = os.getenv("GROQ_API_KEY")
CURRENT_USER_ID = None
INPUT_MODE = "text"

DB_PATH = os.path.join(os.path.dirname(__file__), "DB", "database.db")
os.makedirs(os.path.dirname(DB_PATH), exist_ok=True) # Ensure DB directory exists
database.get_connection = lambda: sqlite3.connect(DB_PATH, check_same_thread=False)
database.initialize_database()

# Global variables to manage conversation flow.
conversation_state = "awaiting_mobile"  # States: awaiting_mobile, awaiting_user_details, awaiting_order
mobile_number = None
# Use separate chat histories for each agent type if running concurrently or switching
chat_history_cafe = []
chat_history_pa = []
chat_history_nurse = []

# Define Audio Directory Consistently (using forward slashes recommended)
AUDIO_DIR = os.path.abspath("./responses_output").replace("\\", "/")
os.makedirs(AUDIO_DIR, exist_ok=True)
# Set the AUDIO_DIR in a2f_utils if it relies on it (or pass it)
a2f_utils.AUDIO_DIR = AUDIO_DIR

app = Flask(__name__, template_folder=".", static_folder="assets", static_url_path="/assets")

# ------------------ Routes ------------------

@app.route("/")
def home():
    return render_template("./assets/html/index.html")

@app.route("/agents")
def agents():
    return render_template("./assets/html/agents.html")

@app.route("/ar")
def index():
    global chat_history_cafe # Reset cafe history on accessing the page
    chat_history_cafe = []
    return render_template("./assets/html/agenticReciptionist.html")

@app.route("/pa")
def personal_assistant():
    global chat_history_pa # Reset PA history
    chat_history_pa = []
    return render_template("./assets/html/pa.html")

@app.route("/agenticNurse")
def agentic_nurse():
    global chat_history_nurse # Reset Nurse history
    chat_history_nurse = []
    return render_template("./assets/html/agenticNurse.html")

# Start of the Cafe Agent
agent_executor = create_cafe_kiosk_agent()

@app.route("/welcome", methods=["GET"])
def welcome():
    global conversation_state, chat_history_cafe
    conversation_state = "awaiting_mobile" # Reset state on welcome
    chat_history_cafe = []
    welcome_message = "Welcome to Cafe {Placeholder}! Please provide your mobile number."
    process_response_text_generic(
        response_text=welcome_message,
        prefix="welcome_segment",
        a2f_base_url=a2f_utils.A2F_BASE_URL, 
        audio_dir=AUDIO_DIR,
        send_audio_func=send_audio_to_audio2face,
        predict_emotion_func=predict_emotion,
        generate_payload_func=generate_emotion_payload_from_probabilities
    )
    return jsonify({"response": welcome_message})

@app.route("/chat", methods=["POST"])
def chat():
    global conversation_state, CURRENT_USER_ID, mobile_number, chat_history_cafe
    data = request.get_json()
    user_input = data.get("user_input", "").strip()
    if not user_input:
        return jsonify({"error": "No input provided"}), 400

    if conversation_state == "awaiting_mobile":
        mobile_number = user_input
        user = database.get_user_by_phone(mobile_number)
        if user:
            CURRENT_USER_ID = user[1]
            conversation_state = "awaiting_order"
            response_text = f"Welcome back, {user[2]}! What would you like today?<br/>{show_menu.invoke('')}"
        else:
            conversation_state = "awaiting_user_details"
            response_text = ("It seems you're a new user. Please provide your details "
                             "in the format: first_name,last_name,email")
        # Process response for TTS/A2F
        process_response_text(response_text) # Use cafe_receptionist's process function
        return jsonify({"response": response_text})

    elif conversation_state == "awaiting_user_details":
        try:
            first_name, last_name, email = [s.strip() for s in user_input.split(",")]
        except Exception:
            response_text = "Invalid format. Please provide details in the format: first_name,last_name,email"
            process_response_text(response_text)
            return jsonify({"response": response_text})
        CURRENT_USER_ID = database.get_next_user_id()
        database.add_user(CURRENT_USER_ID, first_name, last_name, email, mobile_number)
        conversation_state = "awaiting_order"
        response_text = f"User registered successfully, welcome {first_name}! What would you like today?<br/>{show_menu.invoke('')}"
        process_response_text(response_text)
        return jsonify({"response": response_text})

    elif conversation_state == "awaiting_order":
        from langchain_core.messages import HumanMessage, AIMessage
        if user_input.lower().startswith("yes,"):
            from cafe_receptionist import confirm_order
            import cafe_receptionist
            cafe_receptionist.CURRENT_USER_ID = CURRENT_USER_ID

            result_text = confirm_order(user_input)
            process_response_text(result_text) # Process confirmation/error message
            chat_history_cafe.extend([HumanMessage(content=user_input), AIMessage(content=result_text)])
            return jsonify({"response": result_text})
        else:
            # Ensure chat history is passed correctly
            result = agent_executor.invoke({"input": user_input, "chat_history": chat_history_cafe})
            response_text = result["output"]
            process_response_text(response_text) # Process agent's response
            # Split the response into segments for display if needed, emotion check is inside process_response_text
            segments = re.split(r'(?<=[.!?])\s+', response_text)
            segments = [seg for seg in segments if seg.strip()]
            # Update chat history *after* successful invocation and processing
            chat_history_cafe.extend([HumanMessage(content=user_input), AIMessage(content=response_text)])
            display_response = "<br/>".join(segments) # Use segmented for display if preferred
            return jsonify({"response": display_response}) # Send segmented or full back
    else:
        response_text = "Unexpected conversation state."
        process_response_text(response_text) # Process error message
        return jsonify({"response": response_text})

@app.route("/chef_dashboard")
def chef_dashboard():
    conn = database.get_connection()
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM Users")
    users = cursor.fetchall()
    cursor.execute("SELECT * FROM PurchaseHistory")
    purchase_history = cursor.fetchall()
    conn.close()
    return render_template("./assets/html/chef_dashboard.html", users=users, purchase_history=purchase_history)

# Start of the PA Agent
pa_agent = create_pa_agent()

@app.route("/welcome_pa", methods=["GET"])
def welcome_pa():
    global chat_history_pa
    chat_history_pa = [] # Reset PA history on welcome
    welcome_message = "Hello! Welcome to the MetaHuman experience. I’m your assistant. I can help with weather, music, emails, news, or just chat. I’ll watch your mood and adjust if you seem unhappy!"
    process_response_text_generic(
        response_text=welcome_message,
        prefix="welcome_pa_segment",
        a2f_base_url=a2f_utils.A2F_BASE_URL,
        audio_dir=AUDIO_DIR,
        send_audio_func=send_audio_to_audio2face,
        predict_emotion_func=predict_emotion,
        generate_payload_func=generate_emotion_payload_from_probabilities
    )
    return jsonify({"response": welcome_message})

@app.route("/chat_pa", methods=["POST"])
def chat_pa():
    global chat_history_pa
    data = request.get_json()
    user_input = data.get("user_input", "").strip()
    if not user_input:
        return jsonify({"error": "No input provided"}), 400

    # Use the PA agent to process the input
    result = pa_agent.invoke({"input": user_input, "chat_history": chat_history_pa})
    response_text = result["output"]
    from langchain_core.messages import HumanMessage, AIMessage
    chat_history_pa.extend([HumanMessage(content=user_input), AIMessage(content=response_text)])

    # Use the PA specific processing function if it exists, otherwise use generic
    # Assuming personal_assistant.py has process_response_text_pa that calls generic
    from personal_assistant import process_response_text_pa
    process_response_text_pa(response_text)

    # Emotion logging is now inside process_response_text_generic called by process_response_text_pa
    # segments = re.split(r'(?<=[.!?])\s+', response_text)
    # segments = [seg for seg in segments if seg.strip()]
    # for i, segment in enumerate(segments, start=1):
    #     prob, emotion_label = predict_emotion(segment)
    #     print(f"[PA] Segment {i}: \"{segment}\" - Emotion: {emotion_label} with probabilities: {prob}") # Already printed inside generic func

    return jsonify({"response": response_text})

# Nurse Agent Routes
nurse_agent = create_nurse_agent()

@app.route("/welcome_na", methods=["GET"])
def welcome_na():
    global chat_history_nurse
    chat_history_nurse = [] # Reset nurse history on welcome
    welcome_message = "Hello I am your Nurse Assistant!I can assist you in providing patient data,medicine stock CRUD  operations,or my insights in patient diagnostics. Happy to answer your query!"
    process_response_text_generic(
        response_text=welcome_message,
        prefix="welcome_na_segment",
        a2f_base_url=a2f_utils.A2F_BASE_URL,
        audio_dir=AUDIO_DIR,
        send_audio_func=send_audio_to_audio2face,
        predict_emotion_func=predict_emotion,
        generate_payload_func=generate_emotion_payload_from_probabilities
    )
    return jsonify({"response": welcome_message})

@app.route("/chat_na", methods=["POST"])
def chat_na():
    global chat_history_nurse
    data = request.get_json()
    user_input = data.get("user_input", "").strip()
    if not user_input:
        return jsonify({"error": "No input provided"}), 400

    # Use the Nurse agent to process the input
    result = nurse_agent.invoke({"input": user_input, "chat_history": chat_history_nurse})
    response_text = result["output"]

    # Process the response text using the nurse agent's segmentation function
    process_response_text_nurse(response_text)

    # Update nurse agent chat history using the correct format (list of dicts)
    # Check if chat_history_nurse was initialized correctly (should be a list)
    if not isinstance(chat_history_nurse, list):
        chat_history_nurse = [] # Re-initialize if it got corrupted
    chat_history_nurse.extend([{"role": "user", "content": user_input}, {"role": "assistant", "content": response_text}])

    return jsonify({"response": response_text})


# --- Initial A2F Setup ---
print("--- Attempting Initial Audio2Face Setup ---")
try:
    print("1. Loading USD file...")
    load_usd_file()
    print("2. Activating StreamLivelink...")
    activate_stream_livelink()
    print("3. Setting StreamLivelink settings (Enable Audio Stream)...")
    set_stream_livelink_settings()
    print("4. Setting Audio Looping to False...")
    set_audio_looping()
    # --- ADDED CALL TO SET ROOT PATH ---
    print(f"5. Setting Audio2Face Root Path to: {AUDIO_DIR}...")
    set_audio2face_root_path()
    # ------------------------------------
    print("--- Initial Audio2Face Setup Commands Sent ---")
    print("*** IMPORTANT: Ensure Audio2Face app is running and accessible at http://localhost:8011 ***")
except Exception as e:
    print(f"--- ERROR during Initial Audio2Face Setup: {e} ---")
    print("*** Audio2Face features may not work correctly. ***")
    print("*** Please ensure the Audio2Face application is running and accessible at http://localhost:8011 ***")
# --- End Initial A2F Setup ---


if __name__ == "__main__":
    # Note: `debug=True` causes the app to reload, running the setup block twice.
    # Set debug=False for production or when testing single setup execution.
    app.run(host="0.0.0.0", port=5000, debug=True)

```

## Source file: nurse_agent.py

{
  "path": "nurse_agent.py",
  "blob_sha": "5c9c10b0b1f9314c8bae50a5c0633f3f744ef2de",
  "sha256": "065586868777d443849aa11e6e6728664ac64f028ea94cd5959ef47e4cb55cba",
  "lines": 200,
  "bytes": 7834,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 3,
    "face": 2
  },
  "url": "https://github.com/parthubhe/Agentic_MetaHumans/blob/43434f137dd68e6b0e41c13b7ed34e25b4e9db04/nurse_agent.py",
  "function_signature_candidates": [
    {
      "line": 26,
      "text": "def sensor_data_tool(input: str = \"\") -> str:"
    },
    {
      "line": 37,
      "text": "def patient_info_tool(input: str = \"\") -> str:"
    },
    {
      "line": 52,
      "text": "def nurse_alert_tool(input: str) -> str:"
    },
    {
      "line": 70,
      "text": "def dashboard_tool(input: str = \"\") -> str:"
    },
    {
      "line": 82,
      "text": "def med_stock_dashboard_tool(input: str = \"\") -> str:"
    },
    {
      "line": 94,
      "text": "def update_medication_stock_tool(input: str) -> str:"
    },
    {
      "line": 122,
      "text": "def create_nurse_agent() -> AgentExecutor:"
    },
    {
      "line": 177,
      "text": "def process_response_text_nurse(response_text: str, default_delay: float = 1.0):"
    }
  ]
}

```text

import os

from langchain.agents import tool, AgentExecutor
from langchain.prompts import ChatPromptTemplate, MessagesPlaceholder
from langchain.agents.output_parsers.openai_tools import OpenAIToolsAgentOutputParser
from langchain.agents.format_scratchpad.openai_tools import format_to_openai_tool_messages

from Utils.a2f_utils import send_audio_to_audio2face
from Utils.emoClassier import predict_emotion, generate_emotion_payload_from_probabilities

from NA_Utils import nurse_alert, sensor_listener, med_stock_dashboard, dashboard_display, med_stock_handler, \
    patient_data

# Import universal audio processing helper
from Utils.audio_processing_utils import process_response_text_generic

# Directory to store audio responses (same as other agents)
AUDIO_DIR = r"./responses_output"
os.makedirs(AUDIO_DIR, exist_ok=True)
A2F_BASE_URL = "http://localhost:8011"


# ------------------ Nurse Agent Tools ------------------

@tool
def sensor_data_tool(input: str = "") -> str:
    """
    Retrieves simulated sensor data: heart rate, SpO2, and temperature.
    """
    heart_rate = sensor_listener.get_heart_rate()
    spo2 = sensor_listener.get_spo2()
    temperature = sensor_listener.get_temperature()
    return f"Heart Rate: {heart_rate} BPM, SpO2: {spo2}%, Temperature: {temperature:.2f}°F"


@tool
def patient_info_tool(input: str = "") -> str:
    """
    Loads and returns patient information from the patient database.
    """
    patients = patient_data.load_patients()
    if not patients:
        return "No patient data available."
    response = "Patient Information:\n"
    for name, info in patients.items():
        response += f"{name}: Allergies: {', '.join(info['allergies']) if info['allergies'] else 'None'}, "
        response += f"Medications: {', '.join(info['medications']) if info['medications'] else 'None'}\n"
    return response


@tool
def nurse_alert_tool(input: str) -> str:
    """
    Triggers a nurse alert.
    Expects input in the format "vital_type,value,patient_name".
    """
    try:
        vital_type, value_str, patient_name = [s.strip() for s in input.split(",")]
        value = float(value_str)
    except Exception as e:
        return f"Invalid input format for nurse alert: {e}"
    # Only trigger alert if the query explicitly requests it.
    # (The agent chain should decide when to call this tool.)
    severity = "High" if value < 50 or value > 120 else "Low"
    nurse_alert.alert_nurse(value, vital_type, severity, patient_name)
    return f"Nurse alerted for {patient_name}'s {vital_type}."


@tool
def dashboard_tool(input: str = "") -> str:
    """
    Displays the nurse dashboard with patient vitals.
    """
    sample_patients = {
        "John Doe": {"heart_rate": [70, 72, 75], "spo2": [98, 97, 98], "temperature": [98.6, 98.7, 98.5]}
    }
    dashboard_display.display_dashboard(sample_patients)
    return "Patient dashboard displayed."


@tool
def med_stock_dashboard_tool(input: str = "") -> str:
    """
    Loads medication stock using med_stock_handler and displays a stock dashboard using med_stock_dashboard.
    """
    stock = med_stock_handler.load_medication_stock()
    if not stock:
        return "No medication stock data available."
    med_stock_dashboard.display_medication_stock_dashboard(stock)
    return "Medication stock dashboard displayed."


@tool
def update_medication_stock_tool(input: str) -> str:
    """
    Updates medication stock based on input.
    Expects input in the format: "action,name,quantity"
    where action is 'add' or 'reduce'.
    """
    try:
        action, name, quantity_str = [s.strip() for s in input.split(",")]
        quantity = int(quantity_str)
    except Exception as e:
        return f"Invalid input format for updating medication stock: {e}"

    stock = med_stock_handler.load_medication_stock()
    if action.lower() == "add":
        stock = med_stock_handler.add_medication_stock(stock, name, quantity)
        return f"Added {quantity} units to {name}. Current stock: {stock.get(name, 'N/A')}."
    elif action.lower() == "reduce":
        stock, error = med_stock_handler.reduce_medication_stock(stock, name, quantity)
        if error:
            return error
        else:
            return f"Reduced {quantity} units from {name}. Current stock: {stock.get(name, 'N/A')}."
    else:
        return "Action must be 'add' or 'reduce'."


# ------------------ Creating the Nurse Agent ------------------

def create_nurse_agent() -> AgentExecutor:
    """
    Creates and returns the nurse agent with bound tools.
    """
    from langchain_groq import ChatGroq
    llm = ChatGroq(
        model="llama-3.3-70b-versatile",
        temperature=0,
        max_tokens=500,
        timeout=30,
        max_retries=2,
    )
    # Bind all necessary tools including medication stock management.
    llm_with_tools = llm.bind_tools([
        sensor_data_tool,
        patient_info_tool,
        nurse_alert_tool,
        dashboard_tool,
        med_stock_dashboard_tool,
        update_medication_stock_tool
    ])
    prompt = ChatPromptTemplate.from_messages([
        ("system",
         "You are a helpful nurse assistant. You can provide sensor data, patient information, manage medication stock, "
         "and display dashboards. Only invoke the nurse alert tool when an explicit patient alert is required. "
         "Segment your responses for audio processing."),
        MessagesPlaceholder(variable_name="chat_history"),
        ("user", "{input}"),
        MessagesPlaceholder(variable_name="agent_scratchpad"),
    ])
    agent = (
            {
                "input": lambda x: x["input"],
                "agent_scratchpad": lambda x: format_to_openai_tool_messages(x["intermediate_steps"]),
                "chat_history": lambda x: x["chat_history"],
            }
            | prompt
            | llm_with_tools
            | OpenAIToolsAgentOutputParser()
    )
    agent_executor = AgentExecutor(
        agent=agent,
        tools=[
            sensor_data_tool,
            patient_info_tool,
            nurse_alert_tool,
            dashboard_tool,
            med_stock_dashboard_tool,
            update_medication_stock_tool
        ],
        verbose=True
    )
    return agent_executor


def process_response_text_nurse(response_text: str, default_delay: float = 1.0):
    process_response_text_generic(
        response_text=response_text,
        prefix="nurse_response_segment",
        a2f_base_url=A2F_BASE_URL,
        audio_dir=AUDIO_DIR,
        send_audio_func=send_audio_to_audio2face,
        predict_emotion_func=predict_emotion,
        generate_payload_func=generate_emotion_payload_from_probabilities,
        default_delay=default_delay
    )


if __name__ == "__main__":
    # Example usage (for testing purposes)
    sample_response = ("I can provide you with various tools to assist with patient care. "
                       "1. Sensor Data: I can retrieve simulated sensor data, including heart rate, SpO2, and temperature, using the sensor_data_tool function. "
                       "2. Patient Information: I can load and return patient information from the patient database using the patient_info_tool function. "
                       "3. Nurse Alert: I can trigger a nurse alert by using the nurse_alert_tool function, which expects input in the format 'vital_type,value,patient_name'. "
                       "4. Nurse Dashboard: I can display the nurse dashboard with patient vitals using the dashboard_tool function. "
                       "5. Medication Stock Dashboard: I can display medication stock levels. "
                       "6. Update Medication Stock: I can update medication stock. "
                       "Please let me know how I can assist you further.")
    process_response_text_nurse(sample_response)


```

## Source file: Utils/__init__.py

{
  "path": "Utils/__init__.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/parthubhe/Agentic_MetaHumans/blob/43434f137dd68e6b0e41c13b7ed34e25b4e9db04/Utils/__init__.py",
  "function_signature_candidates": []
}

```text



```

## Source file: assets/js/main.js

{
  "path": "assets/js/main.js",
  "blob_sha": "caa8bf075af3a06bd9911a156c84118278d54377",
  "sha256": "ba15654af33c9f0236c82cfd441305c597de2913f047c337dc7d719ea5e5a7b8",
  "lines": 77,
  "bytes": 2363,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/parthubhe/Agentic_MetaHumans/blob/43434f137dd68e6b0e41c13b7ed34e25b4e9db04/assets/js/main.js",
  "function_signature_candidates": []
}

```text

/* main.js
 *
 * Common site functionality:
 * 1. Removes the preloader on page load.
 * 2. Toggles mobile navigation.
 * 3. Initializes AOS (Animate on Scroll).
 * 4. Adds a scroll-to-top button.
 */

document.addEventListener("DOMContentLoaded", () => {

  // 1. Remove the preloader once the window has fully loaded
  const preloader = document.getElementById("preloader");
  if (preloader) {
    window.addEventListener("load", () => {
      // Remove the preloader element from the DOM
      preloader.remove();
    });
  }

  // 2. Mobile nav toggle
  const navMenu = document.getElementById("navmenu");
  const mobileNavToggle = document.querySelector(".mobile-nav-toggle");

  if (mobileNavToggle) {
    mobileNavToggle.addEventListener("click", function () {
      // Toggle a CSS class on the nav to show/hide it
      navMenu.classList.toggle("navbar-mobile");
      // Toggle icon classes (for example, from "bi-list" to "bi-x")
      this.classList.toggle("bi-list");
      this.classList.toggle("bi-x");
    });
  }

  // If your nav items have dropdowns, you might also handle that:
  // (Uncomment if you have dropdown <li> with a .dropdown > a)
  /*
  const navDropdowns = document.querySelectorAll(".navbar-mobile .dropdown > a");
  navDropdowns.forEach((dropdown) => {
    dropdown.addEventListener("click", function (e) {
      e.preventDefault();
      this.nextElementSibling.classList.toggle("dropdown-active");
    });
  });
  */

  // 3. Initialize AOS (Animate On Scroll) library if you're using it
  if (typeof AOS !== "undefined") {
    AOS.init({
      duration: 800,     // Animation duration in ms
      easing: "ease-in-out",
      once: true,        // Whether animation should happen only once
      mirror: false,     // Whether elements animate out while scrolling past them
    });
  }

  // 4. Scroll-to-top button
  const scrollTopBtn = document.getElementById("scroll-top");
  if (scrollTopBtn) {
    scrollTopBtn.addEventListener("click", () => {
      window.scrollTo({
        top: 0,
        behavior: "smooth"
      });
    });

    // Optional: Show/hide the scroll-top button based on scroll position
    window.addEventListener("scroll", () => {
      if (window.scrollY > 100) {
        scrollTopBtn.classList.add("active");
      } else {
        scrollTopBtn.classList.remove("active");
      }
    });
  }

});


```

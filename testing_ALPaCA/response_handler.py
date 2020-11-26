import json
from selenium.webdriver.chrome.options import Options
from selenium import webdriver
from selenium.webdriver.common.desired_capabilities import DesiredCapabilities


def process_browser_log_entry(entry):
    response = json.loads(entry['message'])['message']
    return response


def get_response_filenames(url):
    resp_files = []
    chromedriver_path = "./venv/chromedriver"
    options = Options()
    options.headless = True
    caps = DesiredCapabilities.CHROME
    caps['goog:loggingPrefs'] = {'performance': 'ALL'}
    driver = webdriver.Chrome(chromedriver_path , desired_capabilities=caps , options=options)
    driver.set_page_load_timeout(10)

    try:
        driver.get(url)
    except:
        return None , True

    browser_log = driver.get_log('performance') 
    events = [process_browser_log_entry(entry) for entry in browser_log]

    for event in events : 
        if 'Network.response' in event['method']:
            if 'response' in event['params']:
                resp_files.append((event['params']['response']['url'] , event['params']['response']['statusText']))
    driver.close()
    resp_files.pop(0)
    return resp_files , False 

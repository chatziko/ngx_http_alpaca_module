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
    driver.delete_all_cookies()

    try:
        driver.get(url)
    except:
        return None , True

    browser_log = driver.get_log('performance')
    events = [process_browser_log_entry(entry) for entry in browser_log]

    data_sizes = []
    for event in events :
        if 'Network.responseReceived' in event['method'] and 'response' in event['params']:
            resp_files.append([event['params']['response']['url'] , event['params']['response']['statusText'], 0, 0])
            data_sizes.append(event['params']['requestId'])


        if 'Network.dataReceived' in event['method'] and 'requestId' in event['params']:
            # print(event['params'])
            for count,req_id in enumerate(data_sizes):
                if event['params']['requestId'] == req_id:
                    resp_files[count][2] += event['params']['dataLength']
                    break

        if 'Network.loadingFinished' in event['method'] and 'requestId' in event['params']:
            for count,req_id in enumerate(data_sizes):
                if event['params']['requestId'] == req_id:
                    resp_files[count][3] += event['params']['encodedDataLength']
                    break

    driver.close()
    return resp_files , False

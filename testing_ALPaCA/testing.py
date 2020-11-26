import os
import pathlib

from utils            import colors
from response_handler import get_response_filenames

methods = {
    'deter_simple'      : '/Deterministic/nginx_simple.conf',
    'deter_inline_all'  : '/Deterministic/nginx_inline_all.conf',
    'deter_inline_some' : '/Deterministic/nginx_inline_some.conf',

    'prob_simple'       : '/Probabilistic/nginx_simple.conf',
    'prob_inline_all'   : '/Probabilistic/nginx_inline_all.conf',
    'prob_inline_some'  : '/Probabilistic/nginx_inline_some.conf',
}

inlines = {
    'deter_simple'      : 0,
    'deter_inline_all'  : 3,
    'deter_inline_some' : 2,

    'prob_simple'       : 0,
    'prob_inline_all'   : 3,
    'prob_inline_some'  : 2,
}

success_msg = {
    True  : "finished {}successfully{}!".format(colors.GREENISH, colors.RESET),
    False : "{}failed{}!".format(colors.RED , colors.RESET)
}

def run_nginx(conf):
    os.system("{0}/../build/nginx-1.18.0/objs/nginx -c {0}{1} >/dev/null 2>&1".format(pathlib.Path(__file__).parent.absolute(),conf))

if __name__ == "__main__":

    for conf_name in methods:
        success = True
        run_nginx(methods[conf_name])
        url = 'http://localhost:8888'
        resp_files , timed_out = get_response_filenames(url)
        if (timed_out):
            print("Connection Timed Out!")
            success = False
        else:
            inl_num = 0
            for resp , status in resp_files:
                if "data:image" in resp:
                    inl_num += 1
                # print(status)
            if inl_num != inlines[conf_name]:
                print("Inlining error in {}. Expected {} inlined objects and received {}.".format(conf_name, inlines[conf_name], inl_num))
                success = False
        print("{:17} : {}".format(conf_name, success_msg[success]))

        os.system("fuser -k 8888/tcp >/dev/null 2>&1")

import os
import pathlib

from utils            import colors
from response_handler import get_response_filenames

methods = {
    'deter_simple'      : '/Deterministic/nginx_simple.conf',
    # 'deter_fake_imgs'   : '/Deterministic/nginx_fake_imgs.conf',
    # 'deter_inline_all'  : '/Deterministic/nginx_inline_all.conf',
    # 'deter_inline_some' : '/Deterministic/nginx_inline_some.conf',

    # 'prob_simple'       : '/Probabilistic/nginx_simple.conf',
    # 'prob_fake_imgs'    : '/Probabilistic/nginx_fake_imgs.conf',
    # 'prob_inline_all'   : '/Probabilistic/nginx_inline_all.conf',
    # 'prob_inline_some'  : '/Probabilistic/nginx_inline_some.conf',
}

inlines = {
    'deter_simple'      : 0,
    'deter_fake_imgs'   : 0,
    'deter_inline_all'  : 3,
    'deter_inline_some' : 2,

    'prob_simple'       : 0,
    'prob_fake_imgs'    : 0,
    'prob_inline_all'   : 3,
    'prob_inline_some'  : 2,
}

fake_imgs = {
    'deter_simple'      : 2,
    'deter_fake_imgs'   : 3,
    'deter_inline_all'  : 0,
    'deter_inline_some' : 0,

    'prob_simple'       : 2,
    'prob_fake_imgs'    : [1,2,3,4],
    'prob_inline_all'   : 0,
    'prob_inline_some'  : 0,
}

success_msg = {
    True  : "finished {}successfully{}!".format(colors.GREENISH, colors.RESET),
    False : "{}failed{}!".format(colors.RED , colors.RESET)
}

"""
grep removes error below:
nginx: [alert] could not open error log file: open() "/var/log/nginx/error.log" failed (13: Permission denied)
"""

def run_nginx(conf):
    os.system("{0}/../build/nginx-1.18.0/objs/nginx -c {0}{1} 2>&1 | grep -v '/var/log/nginx/error.log' ".format(pathlib.Path(__file__).parent.absolute(),conf))


def get_alpaca_target_size(file):
    return int(file.split('alpaca-padding=')[1])


if __name__ == "__main__":

    os.system("fuser -k 8888/tcp >/dev/null 2>&1")

    for conf_name in methods:
        success = True
        run_nginx(methods[conf_name])

        url = 'http://localhost:8888'
        resp_files , timed_out = get_response_filenames(url)

        if (timed_out):
            print("Connection Timed Out!")
            success = False

        else:
            inl_num      = 0
            fake_img_num = 0

            for resp, status, resource_size, transfer_size in resp_files:

                if "data:image" in resp:
                    inl_num += 1

                elif "__alpaca_fake_image.png" in resp:
                    fake_img_num += 1

                try:
                    target_size = get_alpaca_target_size(resp)

                    if resource_size != target_size:
                        print("Error expected sizes defers from real size (expected: {} | real: {})".format(resource_size , target_size) )
                        success = False
                        break
                except:
                    pass

                # print(resp , status , resource_size , transfer_size)
            if success == True and inl_num != inlines[conf_name]:
                print("Inlining error in {}. Expected {} inlined objects and received {}.".format(conf_name, inlines[conf_name], inl_num))
                success = False

            if not isinstance(fake_imgs[conf_name],list):
                if success == True and fake_img_num != fake_imgs[conf_name]:
                    print("Fake images error in {}. Expected {} fake images and received {}.".format(conf_name, fake_imgs[conf_name], fake_img_num))
                    success = False

            elif success == True and fake_img_num not in fake_imgs[conf_name]:
                print("Fake images error in {}. Expected {} fake images and received {}.".format(conf_name, fake_imgs[conf_name], fake_img_num))
                success = False

        print("{:17} : {}".format(conf_name, success_msg[success]))

        os.system("fuser -k 8888/tcp >/dev/null 2>&1")